// STUB — NOT compiled/tested (no Android SDK on this machine).
// Thin shell only: explicit downgrade table (A-102 + A-103).
// The binding reports the ceiling; verify-web shows the final badge.
// Rule: on suspicion DOWNGRADE, never upgrade.

package rise.android.attest

/**
 * Assurance ceiling from shell signals (A-102/A-103).
 *
 * Downgrade table (A-5 style: verdict x key-home x environment -> ceiling):
 *
 * | Play Integrity | Key home        | Mock/GPS/state        | Ceiling       |
 * |----------------|-----------------|-----------------------|---------------|
 * | STRONG         | STRONG_BOX      | clean                 | GOLD_L4_CAND  |
 * | STRONG         | TEE             | clean                 | GOLD_L2_MAX   |
 * | STRONG missing | any             | clean                 | SILVER_MAX    |
 * | BASIC + clean  | any             | clean                 | GOLD_L2_MAX   |
 * | any            | any             | mock ON / GPS off     | SILVER_MAX    |
 * | any            | any             | broken chain / replay | RED           |
 * | NONE           | SOFTWARE        | any                   | RED or SILVER |
 *
 * Notes:
 * - GOLD_L4_CAND means "eligible for L4 if core + sensor path confirm";
 *   the shell never grants L4 by itself.
 * - spec/solutions-platform.md §5 (GPS mock): mock = auto downgrade.
 * - spec/solutions-platform.md §9 (privacy off): no GPS gold; silver max.
 * - spec/solutions-platform.md §10 (old phones): API-level fallback.
 */
object RiseAssurance {

    /** Ceiling the shell reports. verify-web may go lower, never higher. */
    enum class Ceiling {
        GOLD_L4_CANDIDATE,
        GOLD_L2_MAX,
        SILVER_MAX,
        RED,
    }

    /** Environment inputs for A-103 (mock/GPS/off-device checks). */
    data class Environment(
        val mockLocationOn: Boolean = false,
        val locationEnabled: Boolean = true,
        val hashChainBroken: Boolean = false,
        val replaySuspected: Boolean = false,
        val apiLevel: Int = 0,
    )

    /**
     * Decides the ceiling. Order matters: red checks first (fail-closed),
     * then environment caps, then verdict x key-home.
     */
    fun decide(
        signal: RiseIntegrity.IntegritySignal,
        keyHome: RiseKeystore.KeyHome,
        env: Environment,
    ): Ceiling {
        // 1. Broken chain / replay suspicion = red, the binding does not argue.
        if (env.hashChainBroken || env.replaySuspected) {
            return Ceiling.RED
        }
        // 2. Mock ON or location OFF: silver at most (§5, §9).
        if (env.mockLocationOn || !env.locationEnabled) {
            return Ceiling.SILVER_MAX
        }
        // 3. Old phone without hardware signals: silver at most (§10).
        // API 28 (P) is the StrongBox floor; below it no gold claim.
        if (env.apiLevel in 1..27) {
            return Ceiling.SILVER_MAX
        }
        // 4. No integrity signal at all on a software key: red.
        if (signal.deviceVerdict == RiseIntegrity.DeviceVerdict.NONE &&
            keyHome == RiseKeystore.KeyHome.SOFTWARE_FALLBACK
        ) {
            return Ceiling.RED
        }
        // 5. STRONG + StrongBox + clean = L4 candidate (core confirms sensor path).
        if (RiseIntegrity.isStrong(signal) && keyHome == RiseKeystore.KeyHome.STRONG_BOX) {
            return Ceiling.GOLD_L4_CANDIDATE
        }
        // 6. BASIC + clean chain = gold-L2 at most.
        if (RiseIntegrity.isBasicClean(signal)) {
            return Ceiling.GOLD_L2_MAX
        }
        // 7. STRONG missing (or unlicensed app): L2/silver ceiling.
        // Conservative default: silver.
        return Ceiling.SILVER_MAX
    }

    /** Maps the ceiling to the manifest `assurance` string (RISE-01 §5). */
    fun toAssuranceField(ceiling: Ceiling): String = when (ceiling) {
        Ceiling.GOLD_L4_CANDIDATE -> "gold-L4"
        Ceiling.GOLD_L2_MAX -> "gold-L2"
        Ceiling.SILVER_MAX -> "silver"
        Ceiling.RED -> "red"
    }
}
