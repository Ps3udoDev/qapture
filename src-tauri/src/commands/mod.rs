pub mod capture;
pub mod cases;
pub mod data;
pub mod doc_versions;
pub mod evidence;
pub mod export;
mod frame;
pub mod projects;
pub mod settings;
pub mod templates;

// Solo lo leen build.rs (vía include!) y los tests de ACL.
#[cfg(test)]
mod names;
