import { mission } from "sdk";

mission.on_rewards((ctx) => {
    const total = ctx.rewards.berry.total;
    const doubled = ctx.rewards.berry.set_total(total * 2);

    oppw4.trace(`mission_rewards_double berry ${total}->${doubled}`);
});
