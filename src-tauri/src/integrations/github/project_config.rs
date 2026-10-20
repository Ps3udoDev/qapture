//! Validación de la configuración GitHub de un proyecto y su forma para el front.

use std::collections::BTreeMap;

use crate::domain::fields::FieldType;
use crate::dto::{FieldDto, LinkedRepoDto, ProjectGithubDto, ProjectGithubInput};
use crate::error::{AppResult, validation};
use crate::repo::integrations::{LinkedRepo, ProjectGithub, RepoRef};

const MAX_LABELS: usize = 10;
const MAX_LABEL_CHARS: usize = 50;

fn invalid_repo(value: &str) -> crate::error::AppError {
    validation(format!(
        "«{value}» no es un repositorio válido (usa dueño/repositorio)."
    ))
}

fn labels_ok(labels: &[String]) -> bool {
    labels.len() <= MAX_LABELS
        && labels.iter().all(|l| {
            let n = l.chars().count();
            (1..=MAX_LABEL_CHARS).contains(&n) && !l.contains(',')
        })
}

/// Valida en el orden fijo de las Global Constraints: repos válidos → no repetidos → un
/// predeterminado → etiquetas → columna de ruteo → opciones del mapa → repos del mapa →
/// repo de evidencias. Sin columna de ruteo el mapa se descarta.
pub fn validate(input: &ProjectGithubInput, fields: &[FieldDto]) -> AppResult<ProjectGithub> {
    let mut repos: Vec<LinkedRepo> = Vec::with_capacity(input.repos.len());
    for linked in &input.repos {
        let value = linked.full_name.trim();
        let repo = RepoRef::parse(value).ok_or_else(|| invalid_repo(value))?;
        repos.push(LinkedRepo {
            repo,
            is_default: linked.is_default,
            default_labels: linked
                .default_labels
                .iter()
                .map(|l| l.trim().to_string())
                .collect(),
        });
    }
    for (i, linked) in repos.iter().enumerate() {
        let name = linked.repo.full_name();
        if repos[..i]
            .iter()
            .any(|r| r.repo.full_name().eq_ignore_ascii_case(&name))
        {
            return Err(validation(format!(
                "El repositorio «{name}» está repetido."
            )));
        }
    }
    if !repos.is_empty() && repos.iter().filter(|r| r.is_default).count() != 1 {
        return Err(validation("Marca un repositorio como predeterminado."));
    }
    if !repos.iter().all(|r| labels_ok(&r.default_labels)) {
        return Err(validation(
            "Las etiquetas deben tener entre 1 y 50 caracteres, sin comas (máximo 10).",
        ));
    }

    let mut routing_map = BTreeMap::new();
    if let Some(field_id) = &input.routing_field_id {
        let field = fields
            .iter()
            .find(|f| &f.id == field_id && !f.archived)
            .ok_or_else(|| validation("La columna de ruteo no existe en este proyecto."))?;
        if field.field_type != FieldType::List {
            return Err(validation("La columna de ruteo debe ser de tipo Lista."));
        }
        for option in input.routing_map.keys() {
            if !field.options.contains(option) {
                return Err(validation(format!(
                    "«{option}» no es una opción de «{}».",
                    field.label
                )));
            }
        }
        for (option, target) in &input.routing_map {
            let target = target.trim();
            let linked = repos
                .iter()
                .find(|r| r.repo.full_name().eq_ignore_ascii_case(target))
                .ok_or_else(|| validation(format!("«{target}» no está vinculado al proyecto.")))?;
            routing_map.insert(option.clone(), linked.repo.full_name());
        }
    }

    let evidence_repo = match input.evidence_repo.as_deref().map(str::trim) {
        None | Some("") => None,
        Some(value) => Some(RepoRef::parse(value).ok_or_else(|| invalid_repo(value))?),
    };

    Ok(ProjectGithub {
        repos,
        routing_field_id: input.routing_field_id.clone(),
        routing_map,
        evidence_repo,
    })
}

/// La columna de ruteo guardada ya no sirve: no existe, se archivó, dejó de ser lista o
/// alguna clave del mapa ya no es una de sus opciones.
pub fn is_stale(config: &ProjectGithub, fields: &[FieldDto]) -> bool {
    let Some(field_id) = &config.routing_field_id else {
        return false;
    };
    match fields.iter().find(|f| &f.id == field_id) {
        Some(field) => {
            field.archived
                || field.field_type != FieldType::List
                || config
                    .routing_map
                    .keys()
                    .any(|option| !field.options.contains(option))
        }
        None => true,
    }
}

pub fn to_dto(config: &ProjectGithub, fields: &[FieldDto]) -> ProjectGithubDto {
    ProjectGithubDto {
        repos: config
            .repos
            .iter()
            .map(|r| LinkedRepoDto {
                full_name: r.repo.full_name(),
                is_default: r.is_default,
                default_labels: r.default_labels.clone(),
            })
            .collect(),
        routing_field_id: config.routing_field_id.clone(),
        routing_map: config.routing_map.clone(),
        evidence_repo: config.evidence_repo.as_ref().map(RepoRef::full_name),
        routing_stale: is_stale(config, fields),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(id: &str, field_type: FieldType, archived: bool) -> FieldDto {
        FieldDto {
            id: id.into(),
            key: id.into(),
            label: "Capa".into(),
            field_type,
            options: vec!["Front".into(), "Back".into()],
            required: false,
            clear_on_duplicate: false,
            width: 12,
            position: 0,
            archived,
        }
    }

    fn fields() -> Vec<FieldDto> {
        vec![
            field("capa", FieldType::List, false),
            field("texto", FieldType::Text, false),
            field("vieja", FieldType::List, true),
        ]
    }

    fn linked(full_name: &str, is_default: bool) -> LinkedRepoDto {
        LinkedRepoDto {
            full_name: full_name.into(),
            is_default,
            default_labels: vec!["bug".into()],
        }
    }

    fn input() -> ProjectGithubInput {
        ProjectGithubInput {
            repos: vec![
                linked("TEAMS4SOFT/GMX-frontend", true),
                linked("TEAMS4SOFT/GMX-backend", false),
            ],
            routing_field_id: Some("capa".into()),
            routing_map: BTreeMap::from([
                ("Front".to_string(), "teams4soft/gmx-frontend".to_string()),
                ("Back".to_string(), "TEAMS4SOFT/GMX-backend".to_string()),
            ]),
            evidence_repo: Some("TEAMS4SOFT/qa-evidencias".into()),
        }
    }

    fn error(input: ProjectGithubInput) -> String {
        validate(&input, &fields()).unwrap_err().user_message()
    }

    #[test]
    fn valid_config_passes() {
        let config = validate(&input(), &fields()).unwrap();
        assert_eq!(config.repos.len(), 2);
        assert_eq!(config.repos[0].repo.full_name(), "TEAMS4SOFT/GMX-frontend");
        assert!(config.repos[0].is_default);
        assert_eq!(config.repos[0].default_labels, ["bug"]);
        assert_eq!(config.routing_field_id.as_deref(), Some("capa"));
        assert_eq!(
            config.routing_map["Front"], "TEAMS4SOFT/GMX-frontend",
            "el mapa usa el nombre del repo vinculado"
        );
        assert_eq!(
            config.evidence_repo.unwrap().full_name(),
            "TEAMS4SOFT/qa-evidencias"
        );
        let empty = ProjectGithubInput {
            repos: vec![],
            routing_field_id: None,
            routing_map: BTreeMap::new(),
            evidence_repo: None,
        };
        assert_eq!(
            validate(&empty, &fields()).unwrap(),
            ProjectGithub::default()
        );
    }

    #[test]
    fn an_invalid_repo_is_rejected() {
        let mut i = input();
        i.repos[1].full_name = "sin-barra".into();
        assert_eq!(
            error(i),
            "«sin-barra» no es un repositorio válido (usa dueño/repositorio)."
        );
    }

    #[test]
    fn a_repeated_repo_is_rejected_ignoring_case() {
        let mut i = input();
        i.repos[1].full_name = "teams4soft/GMX-FRONTEND".into();
        assert_eq!(
            error(i),
            "El repositorio «teams4soft/GMX-FRONTEND» está repetido."
        );
    }

    #[test]
    fn exactly_one_default_repo_is_required() {
        let mut none = input();
        none.repos[0].is_default = false;
        assert_eq!(error(none), "Marca un repositorio como predeterminado.");
        let mut two = input();
        two.repos[1].is_default = true;
        assert_eq!(error(two), "Marca un repositorio como predeterminado.");
    }

    #[test]
    fn labels_are_limited() {
        let message = "Las etiquetas deben tener entre 1 y 50 caracteres, sin comas (máximo 10).";
        for labels in [
            vec![String::new()],
            vec!["a".repeat(51)],
            vec!["a,b".to_string()],
            (0..11).map(|n| format!("l{n}")).collect(),
        ] {
            let mut i = input();
            i.repos[0].default_labels = labels;
            assert_eq!(error(i), message);
        }
        let mut ok = input();
        ok.repos[0].default_labels = (0..10).map(|n| format!("l{n}")).collect();
        ok.repos[0].default_labels[0] = "ñ".repeat(50);
        assert!(validate(&ok, &fields()).is_ok());
    }

    #[test]
    fn the_routing_field_must_exist_and_be_active() {
        for id in ["no-existe", "vieja"] {
            let mut i = input();
            i.routing_field_id = Some(id.into());
            assert_eq!(error(i), "La columna de ruteo no existe en este proyecto.");
        }
    }

    #[test]
    fn the_routing_field_must_be_a_list() {
        let mut i = input();
        i.routing_field_id = Some("texto".into());
        assert_eq!(error(i), "La columna de ruteo debe ser de tipo Lista.");
    }

    #[test]
    fn map_keys_must_be_options() {
        let mut i = input();
        i.routing_map
            .insert("Móvil".into(), "TEAMS4SOFT/GMX-frontend".into());
        assert_eq!(error(i), "«Móvil» no es una opción de «Capa».");
    }

    #[test]
    fn map_values_must_be_linked_repos() {
        let mut i = input();
        i.routing_map
            .insert("Back".into(), "TEAMS4SOFT/otro".into());
        assert_eq!(error(i), "«TEAMS4SOFT/otro» no está vinculado al proyecto.");
    }

    #[test]
    fn the_evidence_repo_must_be_valid() {
        let mut i = input();
        i.evidence_repo = Some("a/..".into());
        assert_eq!(
            error(i),
            "«a/..» no es un repositorio válido (usa dueño/repositorio)."
        );
    }

    #[test]
    fn without_routing_field_the_map_is_dropped() {
        let mut i = input();
        i.routing_field_id = None;
        assert!(validate(&i, &fields()).unwrap().routing_map.is_empty());
    }

    #[test]
    fn staleness_follows_the_routing_field() {
        let config = validate(&input(), &fields()).unwrap();
        assert!(!is_stale(&config, &fields()));
        let mut archived = fields();
        archived[0].archived = true;
        assert!(is_stale(&config, &archived));
        let mut renamed = fields();
        renamed[0].options = vec!["Frontend".into(), "Back".into()];
        assert!(is_stale(&config, &renamed));
        let mut retyped = fields();
        retyped[0].field_type = FieldType::Text;
        assert!(is_stale(&config, &retyped));
        assert!(is_stale(&config, &fields()[1..]));
        assert!(!is_stale(&ProjectGithub::default(), &[]));
        let dto = to_dto(&config, &archived);
        assert!(dto.routing_stale);
        assert_eq!(dto.repos[0].full_name, "TEAMS4SOFT/GMX-frontend");
        assert_eq!(
            dto.evidence_repo.as_deref(),
            Some("TEAMS4SOFT/qa-evidencias")
        );
        assert_eq!(dto.routing_field_id.as_deref(), Some("capa"));
    }
}
