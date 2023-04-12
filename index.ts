import * as solver from "./pkg";

interface Mask {
    cells: boolean[];
    height: number;
    width: number;
}

interface Part {
    isSolid: boolean;
    color: number;
    compressedMask: Mask;
    uncompressedMask: Mask;
}

interface Constraint {
    compressed: boolean | null;
    onCommandLine: boolean | null;
    bugged: boolean | null;
}

interface Requirement {
    partIndex: number;
    constraint: Constraint;
}

interface GridSettings {
    height: number;
    width: number;
    hasOob: boolean;
    commandLineRow: number;
}

interface Position {
    x: number;
    y: number;
}

interface Location {
    position: Position;
    rotation: number;
}

interface Placement {
    loc: Location;
    compressed: boolean;
}

function* solve(
    parts: Part[],
    requirements: Requirement[],
    gridSettings: GridSettings
): Iterable<Placement> {
    const it = solver.solve(
        solver.SolveArgs.fromJs({
            parts,
            requirements,
            gridSettings,
        })
    );
    for (;;) {
        const solution = it.next();
        if (solution == null) {
            break;
        }
        yield solution.toJs();
    }
}

console.log(
    Array.from(
        solve(
            [
                {
                    isSolid: true,
                    color: 0,
                    compressedMask: {
                        width: 3,
                        height: 3,
                        cells: [
                            true,
                            false,
                            false,
                            true,
                            true,
                            false,
                            true,
                            false,
                            false,
                        ],
                    },
                    uncompressedMask: {
                        width: 3,
                        height: 3,
                        cells: [
                            true,
                            false,
                            false,
                            true,
                            true,
                            false,
                            true,
                            false,
                            false,
                        ],
                    },
                },
            ],
            [
                {
                    partIndex: 0,
                    constraint: {
                        bugged: null,
                        compressed: null,
                        onCommandLine: true,
                    },
                },
            ],
            {
                height: 3,
                width: 3,
                hasOob: false,
                commandLineRow: 3,
            }
        )
    )
);
