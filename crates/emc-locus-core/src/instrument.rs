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

impl InstrumentTransport {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Visa => "visa",
            Self::Gpib => "gpib",
            Self::Serial => "serial",
            Self::TcpIp => "tcp_ip",
            Self::Udp => "udp",
            Self::UsbTmc => "usb_tmc",
            Self::Can => "can",
            Self::Lin => "lin",
            Self::ModbusTcp => "modbus_tcp",
            Self::ModbusRtu => "modbus_rtu",
            Self::Rest => "rest",
            Self::VendorSdk => "vendor_sdk",
            Self::Manual => "manual",
            Self::Simulated => "simulated",
        }
    }
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
