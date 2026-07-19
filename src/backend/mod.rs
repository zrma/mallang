pub mod c;

pub(crate) use c::generate_c_test_runner_from_ir;
pub use c::{generate_c, generate_c_from_ir, CompileError};
