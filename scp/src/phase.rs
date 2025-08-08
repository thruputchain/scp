use derive_more::Display;

#[derive(Display, Debug)]
pub enum SCPPhase {
    Prepare,
    Confirm,
    Externalize,
    Num,
}
