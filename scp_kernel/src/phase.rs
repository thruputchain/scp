use derive_more::Display;

#[derive(Display, Debug, PartialEq)]
pub enum Phase {
    Prepare,
    Confirm,
    Externalize,
    Num,
}
