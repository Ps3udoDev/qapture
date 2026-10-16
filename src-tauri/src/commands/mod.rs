pub mod capture;
pub mod cases;
pub mod doc_versions;
pub mod evidence;
pub mod export;
mod frame;
pub mod projects;
pub mod templates;

// Solo lo leen build.rs (vía include!) y los tests de ACL.
#[cfg(test)]
mod names;
