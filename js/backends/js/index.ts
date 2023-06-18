import type { GridSettings, Part, Requirement, Solution } from "../../solver";
import * as array2d from "./array2d";
import * as internal from "./internal";

export function solve(
    parts: Part[],
    requirements: Requirement[],
    gridSettings: GridSettings,
    spinnableColors: boolean[]
): Iterable<Solution> {
    return internal.solve(
        parts.map((part) => ({
            ...part,
            uncompressedMask: array2d.from(
                part.uncompressedMask.cells,
                part.uncompressedMask.height,
                part.uncompressedMask.width
            ),
            compressedMask: array2d.from(
                part.compressedMask.cells,
                part.compressedMask.height,
                part.compressedMask.width
            ),
        })),
        requirements,
        gridSettings,
        spinnableColors
    );
}
