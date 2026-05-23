pub enum Prot {
    None = 0,
    Read = 1,
    Write = 2,
    Exec = 4,
}

pub enum MapFlags {
    Shared = 1,
    Private = 2,
    Anonymous = 4,
}
