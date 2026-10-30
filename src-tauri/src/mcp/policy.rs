use super::{catalog::Group, tools::Meta};
use crate::{
    repo::agent_activity::{self, NewActivity},
    state::AppState,
};
pub fn check(state: &AppState, group: Option<Group>) -> Result<(), String> {
    let Some(group) = group else {
        return Ok(());
    };
    let prefs = state.settings().agents;
    if !prefs.enabled {
        return Err("Los agentes están desactivados. Actívalos en Ajustes › Agentes.".into());
    }
    if !group.allowed(&prefs) {
        return Err(format!(
            "El permiso «{}» está desactivado en Ajustes › Agentes.",
            group.label()
        ));
    }
    if state.recovery().is_some() && group != Group::Read {
        return Err("Qapture está en modo recuperación: solo se permite leer.".into());
    }
    Ok(())
}
/// Con los agentes apagados no se registra nada: un agente que reintenta en bucle no debe llenar
/// el registro (máximo 2 000 filas) y desplazar la actividad real.
pub fn record(state: &AppState, client: &str, tool: &str, ok: bool, meta: &Meta) {
    if !state.settings().agents.enabled {
        return;
    }
    if let Err(e) = agent_activity::insert(
        &state.db(),
        &NewActivity {
            client: client.into(),
            tool: tool.into(),
            project_id: meta.project_id.clone(),
            case_id: meta.case_id.clone(),
            ok,
            detail: meta.detail.clone(),
        },
    ) {
        eprintln!("No se pudo registrar actividad MCP: {e}");
    }
}
