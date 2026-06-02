    function projectCharacterChangedContext(registryModuleList, ctx) {
        let payloadLoaded = false;
        let payload = null;
        const typedCtx = {
            eventKey: ctx.eventKey,
            payloadJson: ctx.payloadJson,
            mod: ctx.mod,
        };
        Object.defineProperty(typedCtx, "payload", {
            enumerable: true,
            get() {
                if (!payloadLoaded) {
                    payload = ctx.payload;
                    payloadLoaded = true;
                }
                return payload;
            },
        });
        Object.defineProperty(typedCtx, "previous_character", {
            enumerable: true,
            get() {
                const eventPayload = typedCtx.payload || {};
                const characterId = eventPayload.previous_character_id;
                return characterId ? resolveCharacter(registryModuleList, ctx.mod, characterId) : null;
            },
        });
        Object.defineProperty(typedCtx, "current_character", {
            enumerable: true,
            get() {
                const eventPayload = typedCtx.payload || {};
                const characterId = eventPayload.current_character_id;
                return characterId ? resolveCharacter(registryModuleList, ctx.mod, characterId) : null;
            },
        });
        Object.defineProperty(typedCtx, "active_character_ids", {
            enumerable: true,
            get() {
                const eventPayload = typedCtx.payload || {};
                const ids = eventPayload.active_character_ids;
                return Array.isArray(ids) ? freeze(ids.slice()) : freeze([]);
            },
        });
        return typedCtx;
    }

    function projectDifficultyAppliedContext(ctx) {
        let payloadLoaded = false;
        let payload = null;
        const typedCtx = {
            eventKey: ctx.eventKey,
            payloadJson: ctx.payloadJson,
            mod: ctx.mod,
        };
        Object.defineProperty(
            typedCtx,
            "payload",
            payloadProperty(() => {
                if (!payloadLoaded) {
                    payload = ctx.payload || {};
                    payloadLoaded = true;
                }
                return payload;
            })
        );
        Object.defineProperty(
            typedCtx,
            "mission_id",
            valueProperty(() => typedCtx.payload.mission_id ?? null)
        );
        Object.defineProperty(typedCtx, "mode", valueProperty(() => typedCtx.payload.mode ?? null));
        Object.defineProperty(
            typedCtx,
            "difficulty",
            valueProperty(() => typedCtx.payload.difficulty ?? null)
        );
        return typedCtx;
    }

    function projectRankResultContext(ctx) {
        let payloadLoaded = false;
        let payload = null;
        const typedCtx = {
            eventKey: ctx.eventKey,
            payloadJson: ctx.payloadJson,
            mod: ctx.mod,
        };
        Object.defineProperty(
            typedCtx,
            "payload",
            payloadProperty(() => {
                if (!payloadLoaded) {
                    payload = ctx.payload || {};
                    payloadLoaded = true;
                }
                return payload;
            })
        );
        Object.defineProperty(
            typedCtx,
            "rank",
            valueProperty(() =>
                freeze({
                    final: typedCtx.payload.rank ?? "unknown",
                    count: typedCtx.payload.count ?? null,
                    time: typedCtx.payload.time ?? null,
                    merge: typedCtx.payload.merge ?? null,
                })
            )
        );
        Object.defineProperty(
            typedCtx,
            "mission",
            valueProperty(() =>
                freeze({
                    mission_id: typedCtx.payload.mission_id ?? null,
                    mode: typedCtx.payload.mode ?? null,
                    difficulty: typedCtx.payload.difficulty ?? null,
                })
            )
        );
        return typedCtx;
    }

    function projectRewardsEventContext(ctx) {
        let payloadLoaded = false;
        let payload = null;
        const typedCtx = {
            eventKey: ctx.eventKey,
            payloadJson: ctx.payloadJson,
            mod: ctx.mod,
        };
        Object.defineProperty(
            typedCtx,
            "payload",
            payloadProperty(() => {
                if (!payloadLoaded) {
                    payload = ctx.payload || {};
                    payloadLoaded = true;
                }
                return payload;
            })
        );
        Object.defineProperty(typedCtx, "rank", valueProperty(() => typedCtx.payload.rank ?? null));
        Object.defineProperty(typedCtx, "berry", valueProperty(() => typedCtx.payload.berry ?? null));
        Object.defineProperty(typedCtx, "souls", valueProperty(() => freeze([])));
        Object.defineProperty(
            typedCtx,
            "crew_points",
            valueProperty(() => typedCtx.payload.crew_points ?? null)
        );
        Object.defineProperty(
            typedCtx,
            "medals",
            valueProperty(() => {
                const medals = typedCtx.payload.medals;
                return Array.isArray(medals) ? freeze(medals.slice()) : freeze([]);
            })
        );
        Object.defineProperty(
            typedCtx,
            "ranks",
            valueProperty(() => {
                const ranks = [
                    typedCtx.payload.count,
                    typedCtx.payload.time,
                    typedCtx.payload.merge,
                    typedCtx.payload.rank,
                ].filter((value) => value != null);
                return freeze(ranks);
            })
        );
        return typedCtx;
    }

    function projectRewardsItemsContext(ctx) {
        let payloadLoaded = false;
        let payload = null;
        const typedCtx = {
            eventKey: ctx.eventKey,
            payloadJson: ctx.payloadJson,
            mod: ctx.mod,
        };
        Object.defineProperty(
            typedCtx,
            "payload",
            payloadProperty(() => {
                if (!payloadLoaded) {
                    payload = ctx.payload || {};
                    payloadLoaded = true;
                }
                return payload;
            })
        );
        Object.defineProperty(
            typedCtx,
            "entries",
            valueProperty(() => {
                const entries = typedCtx.payload.entries;
                return Array.isArray(entries) ? freeze(entries.slice()) : freeze([]);
            })
        );
        return typedCtx;
    }

    function projectMissionRewardsContext(ctx) {
        let payloadLoaded = false;
        let payload = null;
        const mutations = [];
        const typedCtx = {
            eventKey: ctx.eventKey,
            payloadJson: ctx.payloadJson,
            mod: ctx.mod,
            mutations,
        };
        Object.defineProperty(
            typedCtx,
            "payload",
            payloadProperty(() => {
                if (!payloadLoaded) {
                    payload = ctx.payload || {};
                    payloadLoaded = true;
                }
                return payload;
            })
        );
        Object.defineProperty(typedCtx, "rank", valueProperty(() => typedCtx.payload.rank ?? null));
        Object.defineProperty(
            typedCtx,
            "rewards",
            valueProperty(() => createMissionRewardsView(typedCtx, mutations))
        );
        return typedCtx;
    }

    function createMissionRewardsView(ctx, mutations) {
        const berry = createBerryRewardView(ctx, mutations);
        return {
            berry,
            medals: Array.isArray(ctx.payload.medals) ? freeze(ctx.payload.medals.slice()) : freeze([]),
            crew_points: ctx.payload.crew_points ?? null,
        };
    }

    function createBerryRewardView(ctx, mutations) {
        let total = Number(ctx.payload.berry ?? 0);
        return {
            get total() {
                return total;
            },
            set_total(value) {
                const next = Number(value);
                if (!Number.isFinite(next) || next < 0) {
                    throw new Error("berry total must be a non-negative finite number");
                }
                total = Math.trunc(next);
                ctx.payload.berry = total;
                invokeRegistry(ctx.mod, "sdk.mission.set_reward_berry_total", [total]);
                mutations.push(
                    freeze({
                        kind: "berry.set_total",
                        total,
                    })
                );
                return total;
            },
        };
    }

    function resolveCharacter(registryModuleList, currentMod, characterId) {
        const module = lookupPath("sdk.character");
        if (!module || typeof module.find !== "function") {
            return null;
        }
        const value = module.find(String(characterId));
        return wrapRegistryValue(
            registryModuleList,
            currentMod,
            { kind: "named", name: "sdk.Character" },
            value,
            { namespace: "sdk", importName: "character" }
        );
    }
