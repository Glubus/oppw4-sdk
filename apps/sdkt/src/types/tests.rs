use std::{fs, time::SystemTime};

use sdk_schema::{
    RegistryFunctionDescriptor, RegistryMethodDescriptor, RegistryModuleSchema,
    RegistryMutationDescriptor, RegistryTypeDescriptor, RegistryTypeExtensionDescriptor,
    RegistryTypeRef,
};

use super::{install_types, render, runtime_schemas};

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

    let snapshot = fs::read_to_string(types_root.join("snapshot.d.ts")).expect("snapshot");
    assert!(snapshot.contains("export interface Snapshot"));
    assert!(snapshot.contains("readonly mission: SnapshotMission;"));
    assert!(snapshot.contains("readonly difficulty: SnapshotDifficulty;"));
    assert!(snapshot.contains("readonly player: SnapshotPlayer;"));

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
    assert!(rewards.contains("on_medals(callback: (ctx: RewardsMedalsContext) => void): string;"));
    assert!(!rewards.contains("RewardCommitSnapshot"));

    let mission = fs::read_to_string(types_root.join("mission.d.ts")).expect("mission");
    assert!(mission.contains("export interface MissionRewardsContext"));
    assert!(mission.contains("readonly rewards: MissionRewardsView;"));
    assert!(mission.contains("readonly total: number;"));
    assert!(mission.contains("set_total(total: number): number;"));
    assert!(mission.contains("on_rewards(callback: (ctx: MissionRewardsContext) => void): string;"));

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
    assert!(rank.contains("export interface RankCalcPayload"));
    assert!(rank.contains(
        "on_calc_count(callback: (ctx: RankCalcContext) => RankGrade | null | undefined): string;"
    ));
    assert!(rank.contains(
        "on_calc_time(callback: (ctx: RankCalcContext) => RankGrade | null | undefined): string;"
    ));

    let difficulty = fs::read_to_string(types_root.join("difficulty.d.ts")).expect("difficulty");
    assert!(difficulty.contains("export interface DifficultyAppliedPayload"));
    assert!(difficulty.contains("export interface DifficultyAppliedContext extends Oppw4EventContext<DifficultyAppliedPayload>"));
    assert!(difficulty.contains("mode: string | null;"));

    let index = fs::read_to_string(types_root.join("index.d.ts")).expect("index");
    assert!(index.contains("reference path=\"./character.d.ts\""));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn renders_extension_methods_from_functions_and_mutations() {
    let schema = RegistryModuleSchema::new("sdk", "character")
        .function(
            RegistryFunctionDescriptor::new("replace", RegistryTypeRef::Json)
                .param(
                    "target",
                    RegistryTypeRef::Named {
                        name: "Character".to_string(),
                    },
                )
                .param("payload", RegistryTypeRef::String),
        )
        .type_descriptor(
            RegistryTypeDescriptor::new("Character")
                .field("id", RegistryTypeRef::String)
                .field("total", RegistryTypeRef::I64),
        )
        .type_descriptor(
            RegistryTypeDescriptor::new("CharacterSetTotalPayload")
                .field(
                    "target",
                    RegistryTypeRef::Named {
                        name: "Character".to_string(),
                    },
                )
                .field("value", RegistryTypeRef::I64),
        )
        .mutation(RegistryMutationDescriptor::new(
            "set_total",
            "sdk.character.set_total",
            RegistryTypeRef::Named {
                name: "CharacterSetTotalPayload".to_string(),
            },
        ))
        .extension(
            RegistryTypeExtensionDescriptor::new("sdk.Character")
                .method(RegistryMethodDescriptor::new(
                    "replace_movesets",
                    "replace",
                    RegistryTypeRef::Json,
                ))
                .method(RegistryMethodDescriptor::mutation(
                    "set_total",
                    "set_total",
                    RegistryTypeRef::Void,
                )),
        );

    let rendered = render::render_schema_module(&schema);
    assert!(rendered.contains("replace_movesets(payload: string): JsonValue;"));
    assert!(rendered.contains("set_total(value: number): void;"));
}

#[test]
fn runtime_schema_set_stays_available() {
    let schemas = runtime_schemas();
    assert!(schemas.iter().any(|schema| schema.import_name == "player"));
    assert!(schemas.iter().any(|schema| schema.import_name == "mission"));
}

fn temp_root(label: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    std::env::temp_dir().join(format!("oppw4-sdkt-{label}-{nanos}"))
}
