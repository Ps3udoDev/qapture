pub mod capture;
pub mod cases;
pub mod data;
pub mod doc_versions;
pub mod evidence;
pub mod export;
mod frame;
pub mod github;
pub mod projects;
pub mod settings;
pub mod templates;
pub mod updater;

// Solo lo leen build.rs (vía include!) y los tests de ACL.
pub mod agents;
#[cfg(test)]
mod names;
