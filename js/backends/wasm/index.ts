import * as bindings from "../../../pkg";

import type {
    GridSettings,
    Part,
    Requirement,
    Solution,
    Placement,
} from "../../solver";

export function* solve(
    parts: Part[],
    requirements: Requirement[],
    gridSettings: GridSettings,
    spinnableColors: boolean[]
): Iterable<Solution> {
    const it = bindings.solve(
        bindings.SolveArgs.fromJs({
            parts,
            requirements,
            gridSettings,
            spinnableColors,
        })
    );
    try {
        for (;;) {
            const solution = it.next();
            if (solution == null) {
                break;
            }
            yield solution.toJs();
        }
    } finally {
        it.free();
    }
}

export function placeAll(
    parts: Part[],
    requirements: Requirement[],
    placements: Placement[],
    gridSettings: GridSettings
): (number | undefined)[] | null {
    return bindings.placeAll(
        bindings.PlaceAllArgs.fromJs({
            parts,
            requirements,
            placements,
            gridSettings,
        })
    );
}
