use derive_more::Display;

#[derive(Display)]
pub enum SCPPhase {
    Prepare,
    Confirm,
    Externalize,
    Num,
}
