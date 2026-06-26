#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstrumentTransport {
    Visa,
    Gpib,
    Serial,
    TcpIp,
    Udp,
    UsbTmc,
    Can,
    Lin,
    ModbusTcp,
    ModbusRtu,
    Rest,
    VendorSdk,
    Manual,
    Simulated,
}

pub fn baseline_instrument_transports() -> Vec<InstrumentTransport> {
    use InstrumentTransport::*;

    vec![
        Visa, Gpib, Serial, TcpIp, Udp, UsbTmc, Can, Lin, ModbusTcp, ModbusRtu, Rest, VendorSdk,
        Manual, Simulated,
    ]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UpdatePolicy {
    signed_packages_required: bool,
    offline_install_allowed: bool,
    apply_during_measurement_allowed: bool,
}

impl UpdatePolicy {
    pub fn laboratory_default() -> Self {
        Self {
            signed_packages_required: true,
            offline_install_allowed: true,
            apply_during_measurement_allowed: false,
        }
    }

    pub fn signed_packages_required(&self) -> bool {
        self.signed_packages_required
    }

    pub fn offline_install_allowed(&self) -> bool {
        self.offline_install_allowed
    }

    pub fn apply_during_measurement_allowed(&self) -> bool {
        self.apply_during_measurement_allowed
    }
}
