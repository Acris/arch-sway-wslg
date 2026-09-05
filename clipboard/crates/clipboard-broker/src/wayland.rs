use std::collections::HashMap;
use std::fs::File;
use std::os::fd::AsFd;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

use calloop::generic::Generic;
use calloop::timer::{TimeoutAction, Timer};
use calloop::{Interest, LoopHandle, Mode, PostAction, RegistrationToken};
use clipboard_core::MAX_TEXT_BYTES;
use rustix::pipe::{PipeFlags, pipe_with};
use wayland_client::backend::ObjectId;
use wayland_client::protocol::{wl_callback, wl_registry, wl_seat};
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle, delegate_noop};
use wayland_protocols::ext::data_control::v1::client::{
    ext_data_control_device_v1, ext_data_control_manager_v1, ext_data_control_offer_v1,
    ext_data_control_source_v1,
};

use crate::broker::BrokerState;
use crate::io::{read_available, set_nonblocking, write_with_deadline};

const TEXT_MIME_TYPES: [&str; 4] = [
    "text/plain;charset=utf-8",
    "UTF8_STRING",
    "text/plain",
    "STRING",
];
const SENSITIVE_MIME_TYPES: [&str; 2] = [
    "application/x-kde-passwordManagerHint",
    "x-kde-passwordManagerHint",
];
const TRANSFER_TIMEOUT: Duration = Duration::from_secs(3);
const MAX_ACTIVE_TRANSFERS: usize = 8;
// A replaced source is kept until the compositor cancels it. Two publishes in
// one compositor dispatch cycle can leave two of them waiting; a third is not
// realistic, so the oldest goes then.
const MAX_RETIRING_SOURCES: usize = 2;

#[derive(Debug)]
pub enum WaylandEvent {
    SelectionStarted(u64),
    Text {
        generation: u64,
        text: Vec<u8>,
    },
    /// The selection carries nothing to forward; `error` names a failed read.
    Unusable {
        generation: u64,
        error: Option<String>,
    },
    PublicationSynchronized,
    DeviceLost,
}

pub struct WaylandState {
    handle: LoopHandle<'static, BrokerState>,
    manager: Option<ext_data_control_manager_v1::ExtDataControlManagerV1>,
    seat: Option<wl_seat::WlSeat>,
    device: Option<ext_data_control_device_v1::ExtDataControlDeviceV1>,
    // Every offer is consumed by the selection event that follows it, so this
    // holds at most the regular and the primary offer in flight.
    offers: HashMap<ObjectId, OfferInfo>,
    // The source owning the selection, and the ones it replaced: the compositor
    // may still deliver `send` for those until it has seen the new selection.
    active_source: Option<ActiveSource>,
    retiring_sources: Vec<ActiveSource>,
    // Only the newest selection is ever read; a newer one cancels this read.
    offer_read: Option<OfferRead>,
    transfer_slots: Arc<AtomicUsize>,
    sync_sensitive: bool,
    selection_generation: u64,
}

struct ActiveSource {
    proxy: ext_data_control_source_v1::ExtDataControlSourceV1,
    text: Arc<Vec<u8>>,
}

#[derive(Default)]
struct OfferInfo {
    text_rank: Option<usize>,
    sensitive: bool,
}

struct OfferRead {
    generation: u64,
    buffer: Vec<u8>,
    reader: RegistrationToken,
    deadline: RegistrationToken,
}

impl WaylandState {
    pub fn new(handle: LoopHandle<'static, BrokerState>, sync_sensitive: bool) -> Self {
        Self {
            handle,
            manager: None,
            seat: None,
            device: None,
            offers: HashMap::new(),
            active_source: None,
            retiring_sources: Vec::new(),
            offer_read: None,
            transfer_slots: Arc::default(),
            sync_sensitive,
            selection_generation: 0,
        }
    }

    /// Names the global still missing after the registry roundtrip, if any.
    pub fn missing_global(&self) -> Option<&'static str> {
        if self.manager.is_none() {
            Some("ext_data_control_manager_v1 (Sway 1.11 or newer)")
        } else if self.seat.is_none() {
            Some("wl_seat")
        } else {
            None
        }
    }

    pub fn publish_text(
        &mut self,
        text: Vec<u8>,
        qh: &QueueHandle<BrokerState>,
    ) -> Result<(), String> {
        let manager = self
            .manager
            .as_ref()
            .ok_or("ext-data-control manager is unavailable")?;
        let device = self
            .device
            .as_ref()
            .ok_or("ext-data-control device is unavailable")?;
        let source = manager.create_data_source(qh, ());
        for mime in &TEXT_MIME_TYPES[..3] {
            source.offer((*mime).into());
        }
        device.set_selection(Some(&source));
        // Fence the resulting selection events without a blocking roundtrip.
        // The broker keeps delivery serialized until the current offer is read.
        let connection = Connection::from_backend(
            device
                .backend()
                .upgrade()
                .ok_or("Wayland connection is gone")?,
        );
        connection.display().sync(qh, ());
        // The previous source keeps serving until its `cancelled` arrives.
        if let Some(previous) = self.active_source.replace(ActiveSource {
            proxy: source,
            text: Arc::new(text),
        }) {
            self.retiring_sources.push(previous);
            if self.retiring_sources.len() > MAX_RETIRING_SOURCES {
                self.retiring_sources.remove(0).proxy.destroy();
            }
        }
        Ok(())
    }

    fn initialize_device(&mut self, qh: &QueueHandle<BrokerState>) {
        if self.device.is_some() {
            return;
        }
        let (Some(manager), Some(seat)) = (&self.manager, &self.seat) else {
            return;
        };
        self.device = Some(manager.get_data_device(seat, qh, ()));
    }

    fn begin_selection(&mut self) -> u64 {
        self.selection_generation = self.selection_generation.wrapping_add(1).max(1);
        self.cancel_offer_read();
        self.selection_generation
    }

    fn record_offer(
        &mut self,
        offer: &ext_data_control_offer_v1::ExtDataControlOfferV1,
        mime: &str,
    ) {
        let info = self.offers.entry(offer.id()).or_default();
        if is_sensitive_mime(mime) {
            info.sensitive = true;
        }
        // MIME types and their parameters are case-insensitive; the X11 atoms in
        // the list are unaffected because nobody offers them in another case.
        if let Some(rank) = TEXT_MIME_TYPES
            .iter()
            .position(|candidate| candidate.eq_ignore_ascii_case(mime))
            && info.text_rank.is_none_or(|current| rank < current)
        {
            info.text_rank = Some(rank);
        }
    }

    /// Asks for the offer's text and reads it on the event loop until EOF or the
    /// transfer deadline, whichever comes first.
    fn start_offer_read(
        &mut self,
        offer: &ext_data_control_offer_v1::ExtDataControlOfferV1,
        mime: &'static str,
        generation: u64,
    ) -> Result<(), String> {
        let (reader, writer) = pipe_with(PipeFlags::CLOEXEC).map_err(|error| error.to_string())?;
        set_nonblocking(&reader).map_err(|error| error.to_string())?;
        let reader = self
            .handle
            .insert_source(
                Generic::new(File::from(reader), Interest::READ, Mode::Level),
                move |_, file, broker: &mut BrokerState| match broker
                    .wayland
                    .continue_offer_read(file, generation)
                {
                    OfferProgress::Pending => Ok(PostAction::Continue),
                    OfferProgress::Finished(event) => {
                        if let Some(event) = event {
                            broker.handle_wayland_event(event);
                        }
                        Ok(PostAction::Remove)
                    }
                },
            )
            .map_err(|error| error.to_string())?;
        let deadline = self
            .handle
            .insert_source(
                Timer::from_duration(TRANSFER_TIMEOUT),
                move |_, _, broker: &mut BrokerState| {
                    if let Some(event) = broker.wayland.expire_offer_read(generation) {
                        broker.handle_wayland_event(event);
                    }
                    TimeoutAction::Drop
                },
            )
            .map_err(|error| {
                self.handle.remove(reader);
                error.to_string()
            })?;
        offer.receive(mime.into(), writer.as_fd());
        self.offer_read = Some(OfferRead {
            generation,
            buffer: Vec::new(),
            reader,
            deadline,
        });
        Ok(())
    }

    fn continue_offer_read(&mut self, file: &File, generation: u64) -> OfferProgress {
        let Some(read) = self
            .offer_read
            .as_mut()
            .filter(|read| read.generation == generation)
        else {
            return OfferProgress::Finished(None);
        };
        let finished = match read_available(file, &mut read.buffer, MAX_TEXT_BYTES) {
            Ok(false) => return OfferProgress::Pending,
            Ok(true) => None,
            Err(error) => Some(error.to_string()),
        };
        let read = self.offer_read.take().expect("checked offer read");
        self.handle.remove(read.deadline);
        OfferProgress::Finished(Some(match finished {
            None => WaylandEvent::Text {
                generation,
                text: read.buffer,
            },
            Some(error) => WaylandEvent::Unusable {
                generation,
                error: Some(error),
            },
        }))
    }

    fn expire_offer_read(&mut self, generation: u64) -> Option<WaylandEvent> {
        let read = self
            .offer_read
            .take_if(|read| read.generation == generation)?;
        self.handle.remove(read.reader);
        Some(WaylandEvent::Unusable {
            generation,
            error: Some("Wayland clipboard read timed out".into()),
        })
    }

    fn cancel_offer_read(&mut self) {
        if let Some(read) = self.offer_read.take() {
            self.handle.remove(read.reader);
            self.handle.remove(read.deadline);
        }
    }

    fn source_text(
        &self,
        source: &ext_data_control_source_v1::ExtDataControlSourceV1,
    ) -> Option<Arc<Vec<u8>>> {
        self.active_source
            .iter()
            .chain(&self.retiring_sources)
            .find(|candidate| candidate.proxy.id() == source.id())
            .map(|candidate| Arc::clone(&candidate.text))
    }

    fn forget_source(&mut self, source: &ext_data_control_source_v1::ExtDataControlSourceV1) {
        self.active_source
            .take_if(|candidate| candidate.proxy.id() == source.id());
        self.retiring_sources
            .retain(|candidate| candidate.proxy.id() != source.id());
    }
}

fn is_sensitive_mime(mime: &str) -> bool {
    SENSITIVE_MIME_TYPES
        .iter()
        .any(|candidate| candidate.eq_ignore_ascii_case(mime))
}

enum OfferProgress {
    Pending,
    Finished(Option<WaylandEvent>),
}

impl Dispatch<wl_registry::WlRegistry, ()> for BrokerState {
    fn event(
        broker: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            match interface.as_str() {
                "ext_data_control_manager_v1" if broker.wayland.manager.is_none() => {
                    broker.wayland.manager = Some(registry.bind(name, version.min(1), qh, ()));
                }
                "wl_seat" if broker.wayland.seat.is_none() => {
                    broker.wayland.seat = Some(registry.bind(name, version.min(1), qh, ()));
                }
                _ => return,
            }
            broker.wayland.initialize_device(qh);
        }
    }
}

impl Dispatch<wl_callback::WlCallback, ()> for BrokerState {
    fn event(
        broker: &mut Self,
        _: &wl_callback::WlCallback,
        event: wl_callback::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_callback::Event::Done { .. } = event {
            broker.handle_wayland_event(WaylandEvent::PublicationSynchronized);
        }
    }
}

delegate_noop!(BrokerState: ignore wl_seat::WlSeat);
delegate_noop!(BrokerState: ext_data_control_manager_v1::ExtDataControlManagerV1);

impl Dispatch<ext_data_control_offer_v1::ExtDataControlOfferV1, ()> for BrokerState {
    fn event(
        broker: &mut Self,
        offer: &ext_data_control_offer_v1::ExtDataControlOfferV1,
        event: ext_data_control_offer_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let ext_data_control_offer_v1::Event::Offer { mime_type } = event {
            broker.wayland.record_offer(offer, &mime_type);
        }
    }
}

impl Dispatch<ext_data_control_device_v1::ExtDataControlDeviceV1, ()> for BrokerState {
    wayland_client::event_created_child!(
        BrokerState,
        ext_data_control_device_v1::ExtDataControlDeviceV1,
        [
            ext_data_control_device_v1::EVT_DATA_OFFER_OPCODE =>
                (ext_data_control_offer_v1::ExtDataControlOfferV1, ()),
        ]
    );

    fn event(
        broker: &mut Self,
        _: &ext_data_control_device_v1::ExtDataControlDeviceV1,
        event: ext_data_control_device_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            ext_data_control_device_v1::Event::DataOffer { id } => {
                broker.wayland.offers.insert(id.id(), OfferInfo::default());
            }
            ext_data_control_device_v1::Event::Selection { id: Some(offer) } => {
                let generation = broker.wayland.begin_selection();
                let info = broker
                    .wayland
                    .offers
                    .remove(&offer.id())
                    .unwrap_or_default();
                broker.handle_wayland_event(WaylandEvent::SelectionStarted(generation));
                let mime = info
                    .text_rank
                    .filter(|_| broker.wayland.sync_sensitive || !info.sensitive)
                    .map(|rank| TEXT_MIME_TYPES[rank]);
                let result = match mime {
                    Some(mime) => broker.wayland.start_offer_read(&offer, mime, generation),
                    None => Ok(()),
                };
                offer.destroy();
                match result {
                    Ok(()) if mime.is_some() => {}
                    Ok(()) => broker.handle_wayland_event(WaylandEvent::Unusable {
                        generation,
                        error: None,
                    }),
                    Err(error) => broker.handle_wayland_event(WaylandEvent::Unusable {
                        generation,
                        error: Some(error),
                    }),
                }
            }
            ext_data_control_device_v1::Event::Selection { id: None } => {
                let generation = broker.wayland.begin_selection();
                broker.handle_wayland_event(WaylandEvent::SelectionStarted(generation));
                broker.handle_wayland_event(WaylandEvent::Unusable {
                    generation,
                    error: None,
                });
            }
            ext_data_control_device_v1::Event::PrimarySelection { id: Some(offer) } => {
                broker.wayland.offers.remove(&offer.id());
                offer.destroy();
            }
            ext_data_control_device_v1::Event::Finished => {
                broker.handle_wayland_event(WaylandEvent::DeviceLost);
            }
            _ => {}
        }
    }
}

impl Dispatch<ext_data_control_source_v1::ExtDataControlSourceV1, ()> for BrokerState {
    fn event(
        broker: &mut Self,
        source: &ext_data_control_source_v1::ExtDataControlSourceV1,
        event: ext_data_control_source_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            ext_data_control_source_v1::Event::Send { fd, .. } => {
                let Some(text) = broker.wayland.source_text(source) else {
                    return;
                };
                let slots = Arc::clone(&broker.wayland.transfer_slots);
                if slots.fetch_add(1, Ordering::AcqRel) >= MAX_ACTIVE_TRANSFERS {
                    slots.fetch_sub(1, Ordering::AcqRel);
                    eprintln!(
                        "clipboard: refused a Wayland paste, {MAX_ACTIVE_TRANSFERS} transfers already running"
                    );
                    return;
                }
                thread::spawn(move || {
                    let file = File::from(fd);
                    if set_nonblocking(&file).is_ok() {
                        let _ = write_with_deadline(&file, &text, TRANSFER_TIMEOUT);
                    }
                    slots.fetch_sub(1, Ordering::AcqRel);
                });
            }
            ext_data_control_source_v1::Event::Cancelled => {
                broker.wayland.forget_source(source);
                source.destroy();
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::is_sensitive_mime;

    #[test]
    fn kde_sensitive_hint_is_case_insensitive() {
        assert!(is_sensitive_mime("application/x-kde-passwordManagerHint"));
        assert!(is_sensitive_mime("APPLICATION/X-KDE-PASSWORDMANAGERHINT"));
        assert!(is_sensitive_mime("X-KDE-PASSWORDMANAGERHINT"));
        assert!(!is_sensitive_mime("application/x-private-selection"));
    }
}
