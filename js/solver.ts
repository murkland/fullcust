import * as bindings from "../pkg";

export interface Mask {
    cells: boolean[];
    height: number;
    width: number;
}

export interface Part {
    isSolid: boolean;
    color: number;
    compressedMask: Mask;
    uncompressedMask: Mask;
}

export interface Constraint {
    compressed: boolean | null;
    onCommandLine: boolean | null;
    bugged: boolean | null;
}

export interface Requirement {
    partIndex: number;
    constraint: Constraint;
}

export interface GridSettings {
    height: number;
    width: number;
    hasOob: boolean;
    commandLineRow: number;
}

export interface Position {
    x: number;
    y: number;
}

export interface Location {
    position: Position;
    rotation: number;
}

export interface Placement {
    loc: Location;
    compressed: boolean;
}

export type Solution = Placement[];

export function* solve(
    parts: Part[],
    requirements: Requirement[],
    gridSettings: GridSettings
): Iterable<Solution> {
    const it = bindings.solve(
        bindings.SolveArgs.fromJs({
            parts,
            requirements,
            gridSettings,
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
): number[] {
    return bindings.placeAll(
        bindings.PlaceAllArgs.fromJs({
            parts,
            requirements,
            placements,
            gridSettings,
        })
    );
}

export function convertParts(
    rawParts: {
        isSolid: boolean;
        color: number;
        compressedMask: number[];
        uncompressedMask: number[];
    }[],
    height: number,
    width: number
): Part[] {
    return rawParts.map(
        ({ isSolid, color, compressedMask, uncompressedMask }) => ({
            isSolid,
            color,
            compressedMask: {
                height,
                width,
                cells: compressedMask.map((v) => !!v),
            },
            uncompressedMask: {
                height,
                width,
                cells: uncompressedMask.map((v) => !!v),
            },
        })
    );
}
