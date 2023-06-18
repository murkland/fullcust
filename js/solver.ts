import * as backend from "./backends/wasm";

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

export function solve(
    parts: Part[],
    requirements: Requirement[],
    gridSettings: GridSettings,
    spinnableColors: boolean[]
): Iterable<Solution> {
    return backend.solve(parts, requirements, gridSettings, spinnableColors);
}

export function placeAll(
    parts: Part[],
    requirements: Requirement[],
    placements: Placement[],
    gridSettings: GridSettings
): (number | undefined)[] {
    return backend.placeAll(parts, requirements, placements, gridSettings);
}

export function convertParts(
    rawParts: {
        name: string;
        nameJa: string;
        isSolid: boolean;
        color: number;
        compressedMask: number[];
        uncompressedMask: number[];
    }[],
    height: number,
    width: number
): (Part & { name: string; nameJa: string })[] {
    return rawParts.map(
        ({
            name,
            nameJa,
            isSolid,
            color,
            compressedMask,
            uncompressedMask,
        }) => ({
            name,
            nameJa,
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
