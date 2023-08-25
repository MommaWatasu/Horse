pub enum Status {
    AtaSrBsy  = 0x80, // Busy
    AtaSrDrdy = 0x40, // Drive ready
    AtaSrDf   = 0x20, // Drive write fault
    AtaSrDsc  = 0x10, // Drive seek complete
    AtaSrDrq  = 0x08, // Data request ready
    AtaSrCorr = 0x04, // Corrected data
    AtaSrIdx  = 0x02, // Index
    AtaSrErr  = 0x01 // Error
}

pub enum Error {
    AtaErBbk   = 0x80, // Bad block
    AtaErUnc   = 0x40, // Uncorrectable data
    AtaErMc    = 0x20, // Media changed
    AtaErIdnf  = 0x10, // ID mark not found
    AtaErMcr   = 0x08, // Media change request
    AtaErAbrt  = 0x04, // Command aborted
    AtaErTk0nf = 0x02, // Track 0 not found
    AtaErAmnf  = 0x01 // No address mark
}

pub enum Command {
    AtaCmdReadPio        = 0x20,
    AtaCmdReadPioExt     = 0x24,
    AtaCmdReadDma        = 0xcb,
    AtaCmdReadDmaExt     = 0x25,
    AtaCmdWritePio       = 0x30,
    AtaCmdWritePioExt    = 0x34,
    AtaCmdWriteDma       = 0xca,
    AtaCmdWriteDmaExt    = 0x35,
    AtaCmdCacheFluxh     = 0xe7,
    AtaCmdCacheFluxhExt  = 0xea,
    AtaCmdPacket         = 0xa0,
    AtaCmdIdentifyPacket = 0xa1,
    AtaCmdIdentify       = 0xec
}

pub enum Identification {
    AtaIdentDevicetype   = 0,
    AtaIdentCylinders    = 2,
    AtaIdentHeads        = 6,
    AtaIdentSectors      = 12,
    AtaIdentSerial       = 20,
    AtaIdentModel        = 54,
    AtaIdentCapabilities = 98,
    AtaIdentFieldvalid   = 106,
    AtaIdentMaxLba       = 120,
    AtaIdentCommandsets  = 164,
    AtaIdentMaxLbaExt    = 200
}

pub enum InterfaceType {
    IdeAta   = 0x00,
    IdeAtapi = 0x01
}

pub enum DriveType {
    AtaMaster = 0x00,
    AtaSlave  = 0x01
}

pub enum Register {
    AtaRegData          = 0x00,
    AtaRegErrorFeatures = 0x01,
    AtaRegSeccount0     = 0x02,
    AtaRegLba0          = 0x03,
    AtaRegLba1          = 0x04,
    AtaRegLba2          = 0x05,
    AtaRegHddevsel      = 0x06,
    AtaRegCommandStatus = 0x07,
    AtaRegSeccount1     = 0x08,
    AtaRegLab3          = 0x09,
    AtaRegLab4          = 0x0a,
    AtaRegLab5          = 0x0b,
    AtaRegControlAltstatus     = 0x0c,
    AtaRegDevaddress    = 0x0d
}

pub enum Channel {
    Primary   = 0x00,
    Secondary = 0x01
}

pub enum Directions {
    Read  = 0x00,
    Write = 0x01
}

#[derive(Default, Copy, Clone)]
pub struct IdeChannelRegister {
    pub base: u16,
    pub ctrl: u16,
    pub bmide: u16,
    pub no_int: u8
}

#[derive(Copy, Clone, Debug)]
pub struct IdeDevice {
    pub reserved: u8,
    pub channel: usize,
    pub drive: u8,
    pub ata_type: u16,
    pub signature: u16,
    pub capabilities: u16,
    pub commandsets: u32,
    pub size: u32,
    pub model: [u8; 41]
}