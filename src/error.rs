use num_derive::FromPrimitive;

#[derive(Clone, Debug, Eq, FromPrimitive, PartialEq)]
pub enum RecordError {
    /// Incorrect authority provided on update or delete
    IncorrectAuthority,

    /// Calculation overflow
    Overflow,
}
impl From<RecordError> for pinocchio::program_error::ProgramError {
    fn from(e: RecordError) -> Self {
        pinocchio::program_error::ProgramError::Custom(e as u32)
    }
}
