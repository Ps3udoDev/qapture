pub const STATUSES: [&str; 6] = ["No ejecutado", "En ejecución", "Por documentar", "Aprobado", "Fallido", "Bloqueado"];
pub const DEFAULT_STATUS: &str = "No ejecutado";

pub fn is_status(value: &str) -> bool {
    STATUSES.contains(&value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_statuses() {
        assert!(is_status("Aprobado"));
        assert!(!is_status("aprobado"));
        assert_eq!(DEFAULT_STATUS, STATUSES[0]);
    }
}
