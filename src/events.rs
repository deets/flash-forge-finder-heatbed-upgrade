use num_enum::{FromPrimitive, IntoPrimitive};
use esp_idf_sys::{self, c_types};
use esp_idf_svc::eventloop::*;


#[repr(i32)]
#[derive(Copy, Clone, Debug, FromPrimitive, IntoPrimitive)]
pub enum HeatbedControllerEvent {
    IO0 = 1 << 0,
    SampleTimer = 1 << 1,
    DisplayTimer = 1 << 2,
    #[default]
    Unknown = 1 << 31,
}


impl EspTypedEventSerializer<HeatbedControllerEvent> for HeatbedControllerEvent {
    fn serialize<R>(
        event: &HeatbedControllerEvent,
        f: impl for<'a> FnOnce(&'a EspEventPostData) -> R,
    ) -> R {
        let v = match event {
            HeatbedControllerEvent::IO0 => 1 << 0,
            HeatbedControllerEvent::SampleTimer => 1 << 1,
            HeatbedControllerEvent::DisplayTimer => 1 << 2,
            HeatbedControllerEvent::Unknown => 1 << 31,
        };
        f(&unsafe { EspEventPostData::new(Self::source(), Some(v), event) })
    }
}

impl EspTypedEventSource for HeatbedControllerEvent {
    fn source() -> *const c_types::c_char {
        b"HEATBED-SERVICE\0".as_ptr() as *const _
    }
}

impl EspTypedEventDeserializer<HeatbedControllerEvent> for HeatbedControllerEvent {
    #[allow(non_upper_case_globals, non_snake_case)]
    fn deserialize<R>(
        data: &esp_idf_svc::eventloop::EspEventFetchData,
        f: &mut impl for<'a> FnMut(&'a HeatbedControllerEvent) -> R,
    ) -> R {
        let event_id = data.event_id as u32;

        let event = if event_id == (1 << 0) {
            HeatbedControllerEvent::IO0
        } else if event_id == (1 << 1) {
            HeatbedControllerEvent::SampleTimer
        } else if event_id == (1 << 2) {
            HeatbedControllerEvent::DisplayTimer
        } else {
            panic!("Unknown event ID: {}", event_id);
        };
        f(&event)
    }
}
