pub enum Prot {
    ProtNone = 0,
    ProtRead = 1,
    ProtWrite = 2,
    ProtExec = 4,
}

pub enum MapFlags {
    MapShared = 1,
    MapPrivate = 2,
    MapAnonymous = 4,
}
