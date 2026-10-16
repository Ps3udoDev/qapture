pub mod cases;
pub mod evidence;
mod frame;
pub mod projects;
pub mod templates;

// Solo lo leen build.rs (vía include!) y los tests de ACL.
#[cfg(test)]
mod names;
