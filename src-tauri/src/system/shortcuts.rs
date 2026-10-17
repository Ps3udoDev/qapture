//! Lógica pura de atajos globales: normalización, validación y aplicación con rollback.
//! El registro real en el SO vive detrás de [`ShortcutRegistrar`].

use crate::error::{AppResult, validation};
use crate::settings::Shortcuts;

/// Combinaciones que Windows se queda para sí, en forma canónica.
const RESERVED: [&str; 9] = [
    "Alt+F4",
    "Ctrl+Alt+Delete",
    "Win+L",
    "Alt+Tab",
    "Ctrl+Esc",
    "Win+D",
    "Win+E",
    "Win+R",
    "Win+Tab",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShortcutAction {
    Toggle,
    Capture,
    Paste,
    NewCase,
}

impl ShortcutAction {
    pub const ALL: [ShortcutAction; 4] = [Self::Toggle, Self::Capture, Self::Paste, Self::NewCase];

    pub fn label(self) -> &'static str {
        match self {
            Self::Toggle => "Mostrar/ocultar",
            Self::Capture => "Capturar pantalla",
            Self::Paste => "Pegar evidencia",
            Self::NewCase => "Nuevo caso",
        }
    }

    fn accel(self, shortcuts: &Shortcuts) -> &str {
        match self {
            Self::Toggle => &shortcuts.toggle,
            Self::Capture => &shortcuts.capture,
            Self::Paste => &shortcuts.paste,
            Self::NewCase => &shortcuts.new_case,
        }
    }
}

fn is_valid_key(key: &str) -> bool {
    let mut chars = key.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => c.is_ascii_alphanumeric(),
        (Some('F' | 'f'), Some(_)) => key[1..]
            .parse::<u8>()
            .is_ok_and(|n| (1..=24).contains(&n) && key[1..].bytes().all(|b| b.is_ascii_digit())),
        _ => false,
    }
}

/// Devuelve la forma canónica (`Ctrl`, `Alt`, `Shift`, `Win` y la tecla) o `None`.
pub fn normalize(raw: &str) -> Option<String> {
    let (mut ctrl, mut alt, mut shift, mut win) = (false, false, false, false);
    let mut key: Option<String> = None;
    for part in raw.split('+').map(str::trim) {
        let flag = match part.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => Some(&mut ctrl),
            "alt" => Some(&mut alt),
            "shift" => Some(&mut shift),
            "win" | "super" | "meta" => Some(&mut win),
            _ => None,
        };
        match flag {
            Some(flag) if *flag => return None,
            Some(flag) => *flag = true,
            None if key.is_none() && is_valid_key(part) => key = Some(part.to_ascii_uppercase()),
            None => return None,
        }
    }
    let key = key?;
    let mut out = String::new();
    for (on, name) in [(ctrl, "Ctrl"), (alt, "Alt"), (shift, "Shift"), (win, "Win")] {
        if on {
            out.push_str(name);
            out.push('+');
        }
    }
    out.push_str(&key);
    Some(out)
}

/// Valida y devuelve los atajos en forma canónica. Un solo error: el primero.
pub fn validate(shortcuts: &Shortcuts) -> AppResult<Shortcuts> {
    let mut canonical = Vec::with_capacity(4);
    for action in ShortcutAction::ALL {
        let label = action.label();
        let accel = normalize(action.accel(shortcuts))
            .ok_or_else(|| validation(format!("«{label}»: combinación no válida.")))?;
        let has_main_modifier = accel.starts_with("Ctrl+")
            || accel.contains("+Alt+")
            || accel.starts_with("Alt+")
            || accel.contains("+Win+");
        if !has_main_modifier {
            return Err(validation(format!(
                "«{label}»: usa al menos Ctrl, Alt o Win."
            )));
        }
        if RESERVED.contains(&accel.as_str()) {
            return Err(validation(format!("«{label}»: Windows reserva {accel}.")));
        }
        canonical.push(accel);
    }
    for i in 0..4 {
        for j in (i + 1)..4 {
            if canonical[i] == canonical[j] {
                return Err(validation(format!(
                    "«{}» y «{}» usan la misma combinación ({}).",
                    ShortcutAction::ALL[i].label(),
                    ShortcutAction::ALL[j].label(),
                    canonical[i]
                )));
            }
        }
    }
    let mut it = canonical.into_iter();
    let mut next = || it.next().unwrap_or_default();
    Ok(Shortcuts {
        toggle: next(),
        capture: next(),
        paste: next(),
        new_case: next(),
    })
}

pub trait ShortcutRegistrar {
    fn register(&mut self, accel: &str) -> Result<(), String>;
    fn unregister(&mut self, accel: &str);
}

/// Cambia solo las combinaciones distintas. Si el SO rechaza alguna, deja el
/// registrador exactamente como estaba.
pub fn apply(
    reg: &mut impl ShortcutRegistrar,
    current: &Shortcuts,
    wanted: &Shortcuts,
) -> AppResult<()> {
    let changed: Vec<(ShortcutAction, &str, &str)> = ShortcutAction::ALL
        .into_iter()
        .map(|a| (a, a.accel(current), a.accel(wanted)))
        .filter(|(_, old, new)| old != new)
        .collect();

    for (_, old, _) in &changed {
        reg.unregister(old);
    }
    let mut added: Vec<&str> = Vec::new();
    for (action, _, new) in &changed {
        if reg.register(new).is_ok() {
            added.push(new);
            continue;
        }
        for accel in added {
            reg.unregister(accel);
        }
        for (_, old, _) in &changed {
            // Mejor esfuerzo: la combinación anterior estaba libre hace un instante.
            let _ = reg.register(old);
        }
        return Err(validation(format!(
            "No se pudo registrar {new} para «{}»: otra aplicación ya la usa. Se conservan los atajos anteriores.",
            action.label()
        )));
    }
    Ok(())
}

/// Registra al arrancar todo lo que se pueda; devuelve lo que el SO rechazó.
pub fn register_all(
    reg: &mut impl ShortcutRegistrar,
    shortcuts: &Shortcuts,
) -> Vec<(ShortcutAction, String)> {
    ShortcutAction::ALL
        .into_iter()
        .filter_map(|action| {
            let accel = action.accel(shortcuts);
            reg.register(accel)
                .err()
                .map(|_| (action, accel.to_string()))
        })
        .collect()
}

pub fn action_for(accel: &str, current: &Shortcuts) -> Option<ShortcutAction> {
    let accel = normalize(accel)?;
    ShortcutAction::ALL
        .into_iter()
        .find(|a| normalize(a.accel(current)).as_deref() == Some(accel.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[derive(Default)]
    struct FakeRegistrar {
        registered: BTreeSet<String>,
        refuse: BTreeSet<String>,
        log: Vec<String>,
    }

    impl ShortcutRegistrar for FakeRegistrar {
        fn register(&mut self, accel: &str) -> Result<(), String> {
            self.log.push(format!("register {accel}"));
            if self.refuse.contains(accel) {
                return Err("ocupada".into());
            }
            self.registered.insert(accel.to_string());
            Ok(())
        }
        fn unregister(&mut self, accel: &str) {
            self.log.push(format!("unregister {accel}"));
            self.registered.remove(accel);
        }
    }

    fn defaults_registered() -> FakeRegistrar {
        let mut reg = FakeRegistrar::default();
        register_all(&mut reg, &Shortcuts::default());
        reg.log.clear();
        reg
    }

    fn message(result: AppResult<Shortcuts>) -> String {
        result.expect_err("debía fallar").user_message()
    }

    #[test]
    fn normalize_gives_the_canonical_form() {
        assert_eq!(normalize("alt+ctrl+q").as_deref(), Some("Ctrl+Alt+Q"));
        assert_eq!(
            normalize(" Control + Shift + f12 ").as_deref(),
            Some("Ctrl+Shift+F12")
        );
        assert_eq!(normalize("Super+Alt+1").as_deref(), Some("Alt+Win+1"));
        assert_eq!(normalize("Ctrl+Alt"), None);
        assert_eq!(normalize("Ctrl+Ctrl+Q"), None);
        assert_eq!(normalize("Ctrl+Alt+Ñ"), None);
        assert_eq!(normalize("Ctrl+Alt+F25"), None);
    }

    #[test]
    fn validate_reports_the_first_problem_with_fixed_messages() {
        let base = Shortcuts::default();

        let s = Shortcuts {
            capture: "Shift+S".into(),
            ..base.clone()
        };
        assert_eq!(
            message(validate(&s)),
            "«Capturar pantalla»: usa al menos Ctrl, Alt o Win."
        );

        let s = Shortcuts {
            toggle: "Alt+F4".into(),
            ..base.clone()
        };
        assert_eq!(
            message(validate(&s)),
            "«Mostrar/ocultar»: Windows reserva Alt+F4."
        );

        let s = Shortcuts {
            paste: "Ctrl+Alt+S".into(),
            ..base.clone()
        };
        assert_eq!(
            message(validate(&s)),
            "«Capturar pantalla» y «Pegar evidencia» usan la misma combinación (Ctrl+Alt+S)."
        );

        let s = Shortcuts {
            new_case: "Ctrl+".into(),
            ..base.clone()
        };
        assert_eq!(
            message(validate(&s)),
            "«Nuevo caso»: combinación no válida."
        );

        assert_eq!(validate(&base).unwrap(), base);
        let s = Shortcuts {
            toggle: "alt+ctrl+q".into(),
            ..base.clone()
        };
        assert_eq!(validate(&s).unwrap(), base);
    }

    #[test]
    fn apply_only_touches_changed_combinations() {
        let mut reg = defaults_registered();
        let current = Shortcuts::default();
        let wanted = Shortcuts {
            toggle: "Ctrl+Shift+F12".into(),
            ..current.clone()
        };

        apply(&mut reg, &current, &wanted).unwrap();

        assert_eq!(
            reg.log,
            ["unregister Ctrl+Alt+Q", "register Ctrl+Shift+F12"]
        );
        assert!(reg.registered.contains("Ctrl+Shift+F12"));
        assert!(!reg.registered.contains("Ctrl+Alt+Q"));
    }

    #[test]
    fn a_refused_combination_rolls_everything_back() {
        let mut reg = defaults_registered();
        reg.refuse.insert("Ctrl+Alt+P".into());
        let current = Shortcuts::default();
        let wanted = Shortcuts {
            toggle: "Ctrl+Shift+F12".into(),
            capture: "Ctrl+Alt+P".into(),
            ..current.clone()
        };

        let err = apply(&mut reg, &current, &wanted).unwrap_err();

        assert_eq!(
            err.user_message(),
            "No se pudo registrar Ctrl+Alt+P para «Capturar pantalla»: otra aplicación ya la usa. Se conservan los atajos anteriores."
        );
        let expected: BTreeSet<String> = ["Ctrl+Alt+Q", "Ctrl+Alt+S", "Ctrl+Alt+V", "Ctrl+Alt+N"]
            .into_iter()
            .map(String::from)
            .collect();
        assert_eq!(reg.registered, expected);
    }

    #[test]
    fn register_all_keeps_going_and_reports_failures() {
        let mut reg = FakeRegistrar::default();
        reg.refuse.insert("Ctrl+Alt+V".into());

        let failed = register_all(&mut reg, &Shortcuts::default());

        assert_eq!(
            failed,
            vec![(ShortcutAction::Paste, "Ctrl+Alt+V".to_string())]
        );
        assert_eq!(reg.registered.len(), 3);
        assert!(!reg.registered.contains("Ctrl+Alt+V"));
    }

    #[test]
    fn action_for_maps_the_pressed_combination() {
        let d = Shortcuts::default();
        assert_eq!(action_for("Ctrl+Alt+S", &d), Some(ShortcutAction::Capture));
        assert_eq!(action_for("Ctrl+Alt+Z", &d), None);
    }
}
