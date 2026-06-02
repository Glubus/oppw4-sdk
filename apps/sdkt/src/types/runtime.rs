pub(super) fn render_known_runtime_module(import_name: &str) -> Option<String> {
    match import_name {
        "player" => Some(render_player_module()),
        "snapshot" => Some(render_snapshot_module()),
        "difficulty" => Some(render_difficulty_module()),
        "rank" => Some(render_rank_module()),
        "rewards" => Some(render_rewards_module()),
        "mission" => Some(render_mission_module()),
        _ => None,
    }
}

pub(super) fn render_global_declarations() -> String {
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

pub(super) fn render_character_module() -> String {
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

fn render_snapshot_module() -> String {
    let mut output = String::new();
    output.push_str("declare module \"sdk\" {\n");
    output.push_str("  export interface SnapshotMission {\n");
    output.push_str("    id?: number | null;\n");
    output.push_str("    mode?: string | null;\n");
    output.push_str("  }\n\n");
    output.push_str("  export interface SnapshotDifficulty {\n");
    output.push_str("    key?: string | null;\n");
    output.push_str("  }\n\n");
    output.push_str("  export interface SnapshotPlayer {\n");
    output.push_str("    active_character_ids: ReadonlyArray<string>;\n");
    output.push_str("  }\n\n");
    output.push_str("  export interface Snapshot {\n");
    output.push_str("    readonly mission: SnapshotMission;\n");
    output.push_str("    readonly difficulty: SnapshotDifficulty;\n");
    output.push_str("    readonly player: SnapshotPlayer;\n");
    output.push_str("  }\n\n");
    output.push_str("  export const snapshot: Snapshot;\n");
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
    output.push_str("  export interface RankCalcPayload {\n");
    output.push_str("    kind: string;\n");
    output.push_str("    caller: string;\n");
    output.push_str("    caller_rva?: number | null;\n");
    output.push_str("    caller_label: string;\n");
    output.push_str("    row?: number | null;\n");
    output.push_str("    row_offset?: number | null;\n");
    output.push_str("    slot?: number | null;\n");
    output.push_str("    selectors?: ReadonlyArray<number>;\n");
    output.push_str("    thresholds?: ReadonlyArray<number>;\n");
    output.push_str("    all_thresholds?: ReadonlyArray<ReadonlyArray<number>>;\n");
    output.push_str("    value_f32?: number | null;\n");
    output.push_str("    value_u32?: number | null;\n");
    output.push_str("    divisor?: number | null;\n");
    output.push_str("    normalized?: number | null;\n");
    output.push_str("    result?: number | null;\n");
    output.push_str("    result_label?: string | null;\n");
    output.push_str("  }\n\n");
    output.push_str(
        "  export interface RankCalcContext extends Oppw4EventContext<RankCalcPayload> {}\n\n",
    );
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
        "    on_calc_count(callback: (ctx: RankCalcContext) => RankGrade | null | undefined): string;\n",
    );
    output.push_str(
        "    on_calc_time(callback: (ctx: RankCalcContext) => RankGrade | null | undefined): string;\n",
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
