use std::{fs, path::Path};

use sdk_bridge::{
    RegistryFunctionDescriptor, RegistryModuleSchema, RegistryTypeDescriptor, RegistryTypeRef,
};

const TYPES_DIR: &str = ".sdkt/types/oppw4";
const PLAYER_SCHEMA_JSON: &str =
    include_str!("../../../sdk/plugins/runtime/src/registry/schemas/player.json");
const DIFFICULTY_SCHEMA_JSON: &str =
    include_str!("../../../sdk/plugins/runtime/src/registry/schemas/difficulty.json");
const RANK_SCHEMA_JSON: &str =
    include_str!("../../../sdk/plugins/runtime/src/registry/schemas/rank.json");
const REWARDS_SCHEMA_JSON: &str =
    include_str!("../../../sdk/plugins/runtime/src/registry/schemas/rewards.json");
const MISSION_SCHEMA_JSON: &str =
    include_str!("../../../sdk/plugins/runtime/src/registry/schemas/mission.json");

pub(crate) fn install_types(root: &Path) -> Result<(), String> {
    let types_dir = root.join(TYPES_DIR);
    fs::create_dir_all(&types_dir).map_err(|error| format!("{}: {error}", types_dir.display()))?;

    let files = render_typescript_files()?;
    for (name, contents) in &files {
        let path = types_dir.join(name);
        fs::write(&path, contents).map_err(|error| format!("{}: {error}", path.display()))?;
    }

    let legacy_root_types = root.join("sdkt-types.d.ts");
    if legacy_root_types.exists() {
        fs::remove_file(&legacy_root_types)
            .map_err(|error| format!("{}: {error}", legacy_root_types.display()))?;
    }

    println!(
        "installed TypeScript declarations in {}",
        types_dir.display()
    );
    Ok(())
}

fn render_typescript_files() -> Result<Vec<(String, String)>, String> {
    let schemas = runtime_schemas()?;
    let mut files = Vec::new();
    let mut refs = vec![
        "globals.d.ts".to_string(),
        "sdk.d.ts".to_string(),
        "character.d.ts".to_string(),
    ];

    files.push(("globals.d.ts".to_string(), render_global_declarations()));
    files.push(("sdk.d.ts".to_string(), render_sdk_default_module(&schemas)));
    files.push(("character.d.ts".to_string(), render_character_module()));

    for schema in &schemas {
        let name = format!("{}.d.ts", schema.import_name);
        refs.push(name.clone());
        files.push((name, render_schema_module(schema)));
    }

    files.push(("index.d.ts".to_string(), render_index_file(&refs)));
    Ok(files)
}

fn render_index_file(files: &[String]) -> String {
    let mut output = String::new();
    for file in files {
        output.push_str(&format!("/// <reference path=\"./{file}\" />\n"));
    }
    output.push_str("export {};\n");
    output
}

fn render_global_declarations() -> String {
    r#"declare global {
  namespace Oppw4 {
    interface ModContext {
      id: string;
      name: string;
      root: string;
      zipRoot: string;
      isZip: boolean;
    }

    interface EventContext<T = unknown> {
      eventKey: string;
      payloadJson: string;
      payload: T;
      mod: ModContext;
    }

    interface EventApi {
      on<T = unknown>(eventKey: string, callback: (ctx: EventContext<T>) => void): string;
    }

    interface RegistryModule {
      providerId: string;
      name: string;
      load: string;
      schema: unknown | null;
    }

    interface RegistryApi {
      modules(): readonly RegistryModule[];
      has(name: string): boolean;
      module(name: string): unknown | null;
    }

    interface Api {
      mod: ModContext;
      events: EventApi;
      registry: RegistryApi;
      on<T = unknown>(eventKey: string, callback: (ctx: EventContext<T>) => void): string;
      trace(message: string): void;
    }
  }

  const oppw4: Oppw4.Api;

  interface Oppw4ModContext extends Oppw4.ModContext {}
  interface Oppw4EventContext<T = unknown> extends Oppw4.EventContext<T> {}
  interface Oppw4RegistryModule extends Oppw4.RegistryModule {}
  interface Oppw4RegistryApi extends Oppw4.RegistryApi {}
  interface Oppw4Api extends Oppw4.Api {}
}

export {};
"#
    .to_string()
}

fn render_sdk_default_module(schemas: &[RegistryModuleSchema]) -> String {
    let mut export_names = schemas
        .iter()
        .map(|schema| schema.import_name.clone())
        .collect::<Vec<_>>();
    export_names.push("character".to_string());
    export_names.sort_unstable();
    export_names.dedup();

    let mut output = String::new();
    output.push_str("declare module \"sdk\" {\n");
    output.push_str("  export type JsonValue = unknown;\n");
    output.push_str("  export type JsonObject = Record<string, unknown>;\n\n");
    output.push_str("  const sdk: {\n");
    for export_name in &export_names {
        let type_name = if export_name == "character" {
            "CharacterNamespace".to_string()
        } else {
            pascal_case(export_name)
        };
        output.push_str(&format!("    {export_name}: {type_name};\n"));
    }
    output.push_str("  };\n");
    output.push_str("  export default sdk;\n");
    output.push_str("}\n");
    output
}

fn render_character_module() -> String {
    let mut output = String::new();
    output.push_str("declare module \"sdk\" {\n");
    output.push_str("  export interface Character {\n");
    output.push_str("    replace_costume(costume: string, patch: JsonObject): void;\n");
    output.push_str("    replaceCostume(costume: string, patch: JsonObject): void;\n");
    output.push_str("  }\n\n");
    output.push_str("  export interface CharacterNamespace {\n");
    output.push_str("    find(characterId: string): Character | null;\n");
    output.push_str("    get(characterId: string): Character;\n");
    output.push_str("  }\n\n");
    output.push_str("  export const character: CharacterNamespace;\n");
    output.push_str("}\n");
    output
}

fn render_schema_module(schema: &RegistryModuleSchema) -> String {
    match schema.import_name.as_str() {
        "player" => return render_player_module(),
        "difficulty" => return render_difficulty_module(),
        "rank" => return render_rank_module(),
        "rewards" => return render_rewards_module(),
        "mission" => return render_mission_module(),
        _ => {}
    }

    let mut output = String::new();
    let module_type = pascal_case(&schema.import_name);
    output.push_str("declare module \"sdk\" {\n");

    for type_descriptor in &schema.types {
        output.push_str(&render_type_descriptor(type_descriptor, 2));
        output.push('\n');
    }

    output.push_str(&format!("  export interface {module_type} {{\n"));
    for function in &schema.functions {
        output.push_str(&render_function_descriptor(function, 4));
        output.push('\n');
    }
    for event in &schema.events {
        output.push_str(&render_event_descriptor(event, 4));
        output.push('\n');
    }
    output.push_str("  }\n\n");
    output.push_str(&format!(
        "  export const {}: {module_type};\n",
        schema.import_name
    ));
    output.push_str("}\n");
    output
}

fn render_player_module() -> String {
    let mut output = String::new();
    output.push_str("declare module \"sdk\" {\n");
    output.push_str("  export interface CharacterChangedPayload {\n");
    output.push_str("    previous_character_id?: string | null;\n");
    output.push_str("    current_character_id: string;\n");
    output.push_str("    active_character_ids: ReadonlyArray<string>;\n");
    output.push_str("  }\n\n");
    output.push_str("  export interface CharacterChangedContext extends Oppw4EventContext<CharacterChangedPayload> {\n");
    output.push_str("    previous_character: Character | null;\n");
    output.push_str("    current_character: Character | null;\n");
    output.push_str("    active_character_ids: ReadonlyArray<string>;\n");
    output.push_str("  }\n\n");
    output.push_str("  export interface Player {\n");
    output.push_str("    active_characters(): ReadonlyArray<string>;\n");
    output.push_str(
        "    on_character_changed(callback: (ctx: CharacterChangedContext) => void): string;\n",
    );
    output.push_str("  }\n\n");
    output.push_str("  export const player: Player;\n");
    output.push_str("}\n");
    output
}

fn render_difficulty_module() -> String {
    let mut output = String::new();
    output.push_str("declare module \"sdk\" {\n");
    output.push_str("  export interface DifficultyAppliedPayload {\n");
    output.push_str("    mission_id?: number | null;\n");
    output.push_str("    mode: string;\n");
    output.push_str("    difficulty: string;\n");
    output.push_str("  }\n\n");
    output.push_str(
        "  export interface DifficultyAppliedContext extends Oppw4EventContext<DifficultyAppliedPayload> {\n",
    );
    output.push_str("    mission_id: number | null;\n");
    output.push_str("    mode: string | null;\n");
    output.push_str("    difficulty: string | null;\n");
    output.push_str("  }\n\n");
    output.push_str("  export interface Difficulty {\n");
    output.push_str("    on_applied(callback: (ctx: DifficultyAppliedContext) => void): string;\n");
    output.push_str(
        "    on_snapshot(callback: (ctx: Oppw4EventContext<JsonObject>) => void): string;\n",
    );
    output.push_str("  }\n\n");
    output.push_str("  export const difficulty: Difficulty;\n");
    output.push_str("}\n");
    output
}

fn render_rank_module() -> String {
    let mut output = String::new();
    output.push_str("declare module \"sdk\" {\n");
    output.push_str("  export type RankGrade = string;\n\n");
    output.push_str("  export interface RankResultPayload {\n");
    output.push_str("    rank: RankGrade;\n");
    output.push_str("    count?: RankGrade | null;\n");
    output.push_str("    time?: RankGrade | null;\n");
    output.push_str("    merge?: RankGrade | null;\n");
    output.push_str("    mission_id?: number | null;\n");
    output.push_str("    mode?: string | null;\n");
    output.push_str("    difficulty?: string | null;\n");
    output.push_str("  }\n\n");
    output.push_str("  export interface RankBreakdown {\n");
    output.push_str("    final: RankGrade;\n");
    output.push_str("    count?: RankGrade | null;\n");
    output.push_str("    time?: RankGrade | null;\n");
    output.push_str("    merge?: RankGrade | null;\n");
    output.push_str("  }\n\n");
    output.push_str("  export interface RankMissionContext {\n");
    output.push_str("    mission_id?: number | null;\n");
    output.push_str("    mode?: string | null;\n");
    output.push_str("    difficulty?: string | null;\n");
    output.push_str("  }\n\n");
    output.push_str(
        "  export interface RankResultContext extends Oppw4EventContext<RankResultPayload> {\n",
    );
    output.push_str("    rank: RankBreakdown;\n");
    output.push_str("    mission: RankMissionContext;\n");
    output.push_str("  }\n\n");
    output.push_str("  export interface Rank {\n");
    output.push_str("    on_result(callback: (ctx: RankResultContext) => void): string;\n");
    output.push_str(
        "    on_snapshot(callback: (ctx: Oppw4EventContext<JsonObject>) => void): string;\n",
    );
    output.push_str(
        "    on_helper_call(callback: (ctx: Oppw4EventContext<JsonObject>) => void): string;\n",
    );
    output.push_str("  }\n\n");
    output.push_str("  export const rank: Rank;\n");
    output.push_str("}\n");
    output
}

fn render_rewards_module() -> String {
    let mut output = String::new();
    output.push_str("declare module \"sdk\" {\n");
    output.push_str("  export type RankGrade = string;\n\n");
    output.push_str("  export interface RewardsEventPayload {\n");
    output.push_str("    rank: RankGrade;\n");
    output.push_str("    count?: RankGrade | null;\n");
    output.push_str("    time?: RankGrade | null;\n");
    output.push_str("    merge?: RankGrade | null;\n");
    output.push_str("    berry?: number | null;\n");
    output.push_str("    crew_points?: number | null;\n");
    output.push_str("  }\n\n");
    output.push_str("  export interface SoulReward {\n");
    output.push_str("    soul_id: string;\n");
    output.push_str("    count: number;\n");
    output.push_str("  }\n\n");
    output.push_str("  export interface MedalReward {\n");
    output.push_str("    medal_id: string;\n");
    output.push_str("    count: number;\n");
    output.push_str("  }\n\n");
    output.push_str("  export interface RewardItem {\n");
    output.push_str("    item_id: number;\n");
    output.push_str("    amount: number;\n");
    output.push_str("    is_new: boolean;\n");
    output.push_str("  }\n\n");
    output.push_str("  export interface RewardsEvent {\n");
    output.push_str("    rank: RankGrade;\n");
    output.push_str("    berry?: number | null;\n");
    output.push_str("    souls?: ReadonlyArray<SoulReward>;\n");
    output.push_str("    medals?: ReadonlyArray<MedalReward>;\n");
    output.push_str("    crew_points?: number | null;\n");
    output.push_str("    ranks?: ReadonlyArray<RankGrade>;\n");
    output.push_str("  }\n\n");
    output.push_str(
        "  export interface RewardsEventContext extends Oppw4EventContext<RewardsEventPayload> {\n",
    );
    output.push_str("    rank: RankGrade | null;\n");
    output.push_str("    berry: number | null;\n");
    output.push_str("    souls: ReadonlyArray<SoulReward>;\n");
    output.push_str("    medals: ReadonlyArray<RewardItem>;\n");
    output.push_str("    crew_points: number | null;\n");
    output.push_str("    ranks: ReadonlyArray<RankGrade>;\n");
    output.push_str("  }\n\n");
    output.push_str("  export interface RewardsMedalsPayload {\n");
    output.push_str("    entries: ReadonlyArray<RewardItem>;\n");
    output.push_str("  }\n\n");
    output.push_str(
        "  export interface RewardsMedalsContext extends Oppw4EventContext<RewardsMedalsPayload> {\n",
    );
    output.push_str("    entries: ReadonlyArray<RewardItem>;\n");
    output.push_str("  }\n\n");
    output.push_str("  export interface Rewards {\n");
    output.push_str("    on_event(callback: (ctx: RewardsEventContext) => void): string;\n");
    output.push_str("    on_medals(callback: (ctx: RewardsMedalsContext) => void): string;\n");
    output.push_str(
        "    on_commit(callback: (ctx: Oppw4EventContext<JsonObject>) => void): string;\n",
    );
    output.push_str("  }\n\n");
    output.push_str("  export const rewards: Rewards;\n");
    output.push_str("}\n");
    output
}

fn render_mission_module() -> String {
    let mut output = String::new();
    output.push_str("declare module \"sdk\" {\n");
    output.push_str("  export type RankGrade = string;\n\n");
    output.push_str("  export interface MissionRewardMedal {\n");
    output.push_str("    item_id: number;\n");
    output.push_str("    amount: number;\n");
    output.push_str("    is_new: boolean;\n");
    output.push_str("  }\n\n");
    output.push_str("  export interface MissionRewardsPayload {\n");
    output.push_str("    rank: RankGrade;\n");
    output.push_str("    count?: RankGrade | null;\n");
    output.push_str("    time?: RankGrade | null;\n");
    output.push_str("    merge?: RankGrade | null;\n");
    output.push_str("    berry?: number | null;\n");
    output.push_str("    crew_points?: number | null;\n");
    output.push_str("    medals: ReadonlyArray<MissionRewardMedal>;\n");
    output.push_str("  }\n\n");
    output.push_str("  export interface MissionBerryReward {\n");
    output.push_str("    readonly total: number;\n");
    output.push_str("    set_total(total: number): number;\n");
    output.push_str("  }\n\n");
    output.push_str("  export interface MissionRewardsView {\n");
    output.push_str("    readonly berry: MissionBerryReward;\n");
    output.push_str("  }\n\n");
    output.push_str("  export interface MissionRewardMutation {\n");
    output.push_str("    kind: \"berry.set_total\";\n");
    output.push_str("    total: number;\n");
    output.push_str("  }\n\n");
    output.push_str(
        "  export interface MissionRewardsContext extends Oppw4EventContext<MissionRewardsPayload> {\n",
    );
    output.push_str("    readonly rewards: MissionRewardsView;\n");
    output.push_str("    readonly mutations: ReadonlyArray<MissionRewardMutation>;\n");
    output.push_str("  }\n\n");
    output.push_str("  export interface Mission {\n");
    output.push_str("    on_rewards(callback: (ctx: MissionRewardsContext) => void): string;\n");
    output.push_str("  }\n\n");
    output.push_str("  export const mission: Mission;\n");
    output.push_str("}\n");
    output
}

fn render_type_descriptor(type_descriptor: &RegistryTypeDescriptor, indent: usize) -> String {
    let mut output = String::new();
    let padding = " ".repeat(indent);
    output.push_str(&format!(
        "{padding}export interface {} {{\n",
        type_descriptor.name
    ));
    for field in &type_descriptor.fields {
        output.push_str(&format!(
            "{}{}: {};\n",
            " ".repeat(indent + 2),
            field.name,
            render_type_ref(&field.type_ref)
        ));
    }
    output.push_str(&format!("{padding}}}\n"));
    output
}

fn render_function_descriptor(function: &RegistryFunctionDescriptor, indent: usize) -> String {
    let params = function
        .params
        .iter()
        .map(|param| format!("{}: {}", param.name, render_type_ref(&param.type_ref)))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "{}{}({}): {};",
        " ".repeat(indent),
        function.name,
        params,
        render_type_ref(&function.returns)
    )
}

fn render_event_descriptor(event: &sdk_bridge::RegistryEventDescriptor, indent: usize) -> String {
    format!(
        "{}on_{}(callback: (ctx: Oppw4EventContext<{}>) => void): string;",
        " ".repeat(indent),
        event.name,
        render_type_ref(&event.payload)
    )
}

fn render_type_ref(type_ref: &RegistryTypeRef) -> String {
    match type_ref {
        RegistryTypeRef::Named { name } => ts_type_name(name.split('.').last().unwrap_or(name)),
        RegistryTypeRef::Optional { inner } => {
            format!("{} | null | undefined", render_type_ref(inner))
        }
        RegistryTypeRef::Array { inner } => format!("ReadonlyArray<{}>", render_type_ref(inner)),
        RegistryTypeRef::Void => "void".to_string(),
        RegistryTypeRef::Bool => "boolean".to_string(),
        RegistryTypeRef::I64 | RegistryTypeRef::F64 => "number".to_string(),
        RegistryTypeRef::String => "string".to_string(),
        RegistryTypeRef::Json => "JsonValue".to_string(),
    }
}

fn runtime_schemas() -> Result<Vec<RegistryModuleSchema>, String> {
    [
        PLAYER_SCHEMA_JSON,
        DIFFICULTY_SCHEMA_JSON,
        RANK_SCHEMA_JSON,
        REWARDS_SCHEMA_JSON,
        MISSION_SCHEMA_JSON,
    ]
    .into_iter()
    .map(|schema_json| {
        serde_json::from_str(schema_json)
            .map_err(|error| format!("failed to parse registry schema: {error}"))
    })
    .collect()
}

fn pascal_case(value: &str) -> String {
    let mut output = String::new();
    let parts = value
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if parts.is_empty() {
        return "Sdk".to_string();
    }
    if parts.len() == 1 {
        return preserve_pascal_case(parts[0]);
    }
    for part in parts {
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            output.push(first.to_ascii_uppercase());
            for ch in chars {
                output.push(ch.to_ascii_lowercase());
            }
        }
    }
    if output.is_empty() {
        "Sdk".to_string()
    } else {
        output
    }
}

fn preserve_pascal_case(value: &str) -> String {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return "Sdk".to_string();
    };
    let mut output = String::new();
    output.push(first.to_ascii_uppercase());
    output.extend(chars);
    output
}

fn ts_type_name(value: &str) -> String {
    if value.chars().any(|ch| !ch.is_ascii_alphanumeric()) {
        pascal_case(value)
    } else {
        preserve_pascal_case(value)
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, time::SystemTime};

    use super::*;

    #[test]
    fn installs_split_types_inside_project_root() {
        let root = temp_root("types");
        fs::create_dir_all(&root).expect("root");

        install_types(&root).expect("install types");

        let types_root = root.join(".sdkt").join("types").join("oppw4");
        assert!(types_root.join("index.d.ts").is_file());
        assert!(types_root.join("globals.d.ts").is_file());
        assert!(types_root.join("sdk.d.ts").is_file());
        assert!(types_root.join("character.d.ts").is_file());
        assert!(types_root.join("player.d.ts").is_file());
        assert!(!root.join("sdkt-types.d.ts").exists());

        let character = fs::read_to_string(types_root.join("character.d.ts")).expect("character");
        assert!(character.contains("export interface Character"));
        assert!(character.contains("replace_costume"));
        assert!(character.contains("export const character: CharacterNamespace;"));

        let player = fs::read_to_string(types_root.join("player.d.ts")).expect("player");
        assert!(player.contains("export interface CharacterChangedPayload"));
        assert!(player.contains("current_character_id: string;"));
        assert!(player.contains("export interface CharacterChangedContext extends Oppw4EventContext<CharacterChangedPayload>"));
        assert!(player.contains("previous_character: Character | null;"));
        assert!(player.contains("current_character: Character | null;"));
        assert!(player.contains("active_character_ids: ReadonlyArray<string>;"));

        let sdk = fs::read_to_string(types_root.join("sdk.d.ts")).expect("sdk");
        assert!(sdk.contains("export default sdk;"));

        let rewards = fs::read_to_string(types_root.join("rewards.d.ts")).expect("rewards");
        assert!(rewards.contains("export interface RewardsEventPayload"));
        assert!(rewards.contains("count?: RankGrade | null;"));
        assert!(rewards.contains("time?: RankGrade | null;"));
        assert!(rewards.contains("merge?: RankGrade | null;"));
        assert!(rewards.contains("crew_points?: number | null;"));
        assert!(rewards.contains(
            "export interface RewardsEventContext extends Oppw4EventContext<RewardsEventPayload>"
        ));
        assert!(rewards.contains("berry: number | null;"));
        assert!(rewards.contains("souls: ReadonlyArray<SoulReward>;"));
        assert!(rewards.contains("crew_points: number | null;"));
        assert!(rewards.contains("medals: ReadonlyArray<RewardItem>;"));
        assert!(rewards.contains("export interface RewardsMedalsPayload"));
        assert!(rewards.contains(
            "export interface RewardsMedalsContext extends Oppw4EventContext<RewardsMedalsPayload>"
        ));
        assert!(
            rewards.contains("on_medals(callback: (ctx: RewardsMedalsContext) => void): string;")
        );
        assert!(!rewards.contains("RewardCommitSnapshot"));

        let mission = fs::read_to_string(types_root.join("mission.d.ts")).expect("mission");
        assert!(mission.contains("export interface MissionRewardsContext"));
        assert!(mission.contains("readonly rewards: MissionRewardsView;"));
        assert!(mission.contains("readonly total: number;"));
        assert!(mission.contains("set_total(total: number): number;"));
        assert!(
            mission.contains("on_rewards(callback: (ctx: MissionRewardsContext) => void): string;")
        );

        let rank = fs::read_to_string(types_root.join("rank.d.ts")).expect("rank");
        assert!(rank.contains("export interface RankResultPayload"));
        assert!(rank.contains("count?: RankGrade | null;"));
        assert!(rank.contains("time?: RankGrade | null;"));
        assert!(rank.contains("merge?: RankGrade | null;"));
        assert!(rank.contains(
            "export interface RankResultContext extends Oppw4EventContext<RankResultPayload>"
        ));
        assert!(rank.contains("export interface RankBreakdown"));
        assert!(rank.contains("final: RankGrade;"));
        assert!(rank.contains("count?: RankGrade | null;"));
        assert!(rank.contains("time?: RankGrade | null;"));
        assert!(rank.contains("merge?: RankGrade | null;"));
        assert!(rank.contains("mission: RankMissionContext;"));

        let difficulty =
            fs::read_to_string(types_root.join("difficulty.d.ts")).expect("difficulty");
        assert!(difficulty.contains("export interface DifficultyAppliedPayload"));
        assert!(difficulty.contains("export interface DifficultyAppliedContext extends Oppw4EventContext<DifficultyAppliedPayload>"));
        assert!(difficulty.contains("mode: string | null;"));

        let index = fs::read_to_string(types_root.join("index.d.ts")).expect("index");
        assert!(index.contains("reference path=\"./character.d.ts\""));

        let _ = fs::remove_dir_all(root);
    }

    fn temp_root(label: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("oppw4-sdkt-{label}-{nanos}"))
    }
}
