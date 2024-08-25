use esp32_nimble::{
    enums::*,
    utilities::mutex::Mutex,
    BLEAdvertisementData,
    BLECharacteristic,
    BLEDevice,
    BLEServer,
    utilities::BleUuid,
    NimbleProperties,
};
use std::sync::Arc;
use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::sys::esp_timer_get_time;
use log::info;

const MIDI_SERVICE_UUID: &str = "03B80E5A-EDE8-4B33-A751-6CE34EC4C700";
const MIDI_CHARACTERISTIC_UUID: &str = "7772E5DB-3868-4112-A1A9-F2669D106BF3";

pub struct BLEMIDIDevice {
    server: &'static mut BLEServer,
    midi_characteristic: Arc<Mutex<BLECharacteristic>>,
}

impl BLEMIDIDevice {
    pub fn new() -> anyhow::Result<Self> {
        let midi_service_uuid = BleUuid::from_uuid128_string(MIDI_SERVICE_UUID).unwrap();
        let midi_characteristic_uuid = BleUuid::from_uuid128_string(MIDI_CHARACTERISTIC_UUID).unwrap();

        let device = BLEDevice::take();
        device
            .security()
            .set_auth(AuthReq::all())
            .set_io_cap(SecurityIOCap::NoInputNoOutput)
            .resolve_rpa();

        let server = device.get_server();

        // MIDI Service
        let midi_service = server.create_service(midi_service_uuid);
        let midi_characteristic = midi_service.lock().create_characteristic(
            midi_characteristic_uuid,
            NimbleProperties::READ | NimbleProperties::NOTIFY | NimbleProperties::WRITE_NO_RSP,
        );

        // Advertising
        let ble_advertising = device.get_advertising();
        ble_advertising.lock().set_data(
            BLEAdvertisementData::new()
                .name("ESP32 BLE MIDI Device")
                .add_service_uuid(midi_service_uuid)
                .appearance(0x0508),
        )?;
        ble_advertising.lock().start()?;

        Ok(Self {
            server,
            midi_characteristic,
        })
    }

    fn get_timestamp() -> u32 {
        unsafe { (esp_timer_get_time() / 1000) as u32 }
    }

    fn send_midi_message(&self, message: &[u8]) -> anyhow::Result<()> {
        let timestamp = Self::get_timestamp();
        let header = ((timestamp >> 7) & 0x3F) as u8 | 0x80;
        let timestamp_lsb = (timestamp & 0x7F) as u8 | 0x80;

        let mut midi_packet = vec![header, timestamp_lsb];
        midi_packet.extend_from_slice(message);

        info!("Sending MIDI packet: {:?}", midi_packet);

        self.midi_characteristic.lock().set_value(&midi_packet);
        self.midi_characteristic.lock().notify();

        info!("MIDI packet sent successfully");

        Ok(())
    }

    pub fn connected(&self) -> bool {
        self.server.connected_count() > 0
    }

    pub fn send_note_on(&self, note_number: u8, velocity: u8) -> anyhow::Result<()> {
        info!("Sending Note On");
        self.send_midi_message(&[0x90, note_number, velocity])?;
        Ok(())
    }

    pub fn send_note_off(&self, note_number: u8, velocity: u8) -> anyhow::Result<()> {
        info!("Sending Note Off");
        self.send_midi_message(&[0x80, note_number, velocity])?;
        Ok(())
    }
}
