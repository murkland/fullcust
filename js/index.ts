import { GridSettings, Part, placeAll, Requirement, solve } from "./solver";

const parts: Part[] = [
    {
        isSolid: true,
        color: 0,
        compressedMask: {
            width: 2,
            height: 3,
            cells: [true, false, true, true, true, false],
        },
        uncompressedMask: {
            width: 2,
            height: 3,
            cells: [true, false, true, true, true, false],
        },
    },
];

const requirements: Requirement[] = [
    {
        partIndex: 0,
        constraint: {
            bugged: null,
            compressed: null,
            onCommandLine: true,
        },
    },
    {
        partIndex: 0,
        constraint: {
            bugged: null,
            compressed: null,
            onCommandLine: true,
        },
    },
];

const gridSettings: GridSettings = {
    height: 7,
    width: 7,
    hasOob: true,
    commandLineRow: 3,
};

const solutions = Array.from(solve(parts, requirements, gridSettings));

for (const solution of solutions) {
    const griddy: string[][] = new Array(gridSettings.height)
        .fill(null)
        .map((_) => new Array(gridSettings.width).fill("."));

    const p = placeAll(parts, requirements, solution, gridSettings);

    for (let y = 0; y < gridSettings.height; ++y) {
        for (let x = 0; x < gridSettings.width; ++x) {
            const i = y * gridSettings.width + x;
            griddy[y][x] = p[i] == -1 ? "." : p[i].toString();
        }
    }

    console.log(
        `${JSON.stringify(solution)}\n${griddy
            .map((row) => row.join(" "))
            .join("\n")}`
    );
}
