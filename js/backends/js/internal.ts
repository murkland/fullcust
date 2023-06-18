import type {
    Constraint,
    GridSettings,
    Placement,
    Position,
    Requirement,
    Solution,
} from "../../solver";
import * as array2d from "./array2d";

const Cell = {
    FORBIDDEN: -2,
    EMPTY: -1,
};

interface Part {
    isSolid: boolean;
    color: number;
    compressedMask: array2d.Array2D<boolean>;
    uncompressedMask: array2d.Array2D<boolean>;
}

interface Location {
    position: Position;
    rotation: number;
}

function arrayCountNumber(arr: number[], p: number): number {
    let n = 0;
    for (const v of arr) {
        if (v === p) {
            n += 1;
        }
    }
    return n;
}

function arrayCountTrue(arr: boolean[]): number {
    let n = 0;
    for (const v of arr) {
        if (v) {
            n += 1;
        }
    }
    return n;
}

function arrayAny(arr: boolean[]): boolean {
    for (const v of arr) {
        if (v) {
            return true;
        }
    }
    return false;
}

function arrayBooleanToNumber(arr: boolean[]): number[] {
    const out = new Array(arr.length);
    for (let i = 0; i < arr.length; ++i) {
        out[i] = +arr[i];
    }
    return out;
}

function trim(arr2d: array2d.Array2D<boolean>): array2d.Array2D<boolean> {
    let left = 0;
    for (; left < arr2d.ncols; ++left) {
        if (!arrayAny(array2d.col(arr2d, left))) {
            break;
        }
    }

    let top = 0;
    for (; top < arr2d.nrows; ++top) {
        if (!arrayAny(array2d.row(arr2d, top))) {
            break;
        }
    }

    let right = arr2d.ncols - 1;
    for (; right >= 0; --right) {
        if (!arrayAny(array2d.col(arr2d, right))) {
            break;
        }
    }
    ++right;

    let bottom = arr2d.nrows - 1;
    for (; bottom >= 0; --bottom) {
        if (!arrayAny(array2d.row(arr2d, bottom))) {
            break;
        }
    }
    ++bottom;

    const nrows = bottom - top;
    const ncols = right - left;

    return array2d.subarray(arr2d, top, left, nrows, ncols);
}

class Grid {
    hasOob: boolean;
    commandLineRow: number;
    cells: array2d.Array2D<number>;

    constructor(settings: GridSettings) {
        this.hasOob = settings.hasOob;
        this.commandLineRow = settings.commandLineRow;
        this.cells = array2d.full<number>(
            Cell.EMPTY,
            settings.height,
            settings.width
        );
        if (this.hasOob) {
            this.cells[0 * settings.width + 0] = Cell.FORBIDDEN;
            this.cells[0 * settings.width + (settings.width - 1)] =
                Cell.FORBIDDEN;
            this.cells[
                (settings.height - 1) * settings.width + (settings.width - 1)
            ] = Cell.FORBIDDEN;
            this.cells[(settings.height - 1) * settings.width + 0] =
                Cell.FORBIDDEN;
        }
    }

    getSettings(): GridSettings {
        return {
            width: this.cells.ncols,
            height: this.cells.nrows,
            hasOob: this.hasOob,
            commandLineRow: this.commandLineRow,
        };
    }

    clone(): Grid {
        const grid = new Grid({
            hasOob: this.hasOob,
            commandLineRow: this.commandLineRow,
            width: 0,
            height: 0,
        });
        grid.cells = array2d.copy(this.cells);
        return grid;
    }

    place(mask: array2d.Array2D<boolean>, pos: Position, reqIdx: number) {
        let srcY = 0;
        let dstY = 0;
        if (pos.y < 0) {
            srcY = -pos.y;
        } else {
            dstY = pos.y;
        }

        let srcX = 0;
        let dstX = 0;
        if (pos.x < 0) {
            srcX = -pos.x;
        } else {
            dstX = pos.x;
        }

        for (let y = 0; y < mask.nrows; ++y) {
            for (let x = 0; x < mask.ncols; ++x) {
                if (
                    x >= srcX &&
                    y >= srcY &&
                    x < mask.ncols - dstX &&
                    y < mask.nrows - dstY
                ) {
                    continue;
                }

                if (mask[y * mask.ncols + x]) {
                    return false;
                }
            }
        }

        // Actually do the placement...
        const copyNrows = Math.min(mask.nrows - srcY, this.cells.nrows - dstY);
        const copyNcols = Math.min(mask.ncols - srcX, this.cells.ncols - dstX);

        // TODO

        return true;
    }
}

interface Candidate {
    placement: Placement;
    mask: array2d.Array2D<boolean>;
}

function partsArr2DForGrid(
    grid: Grid,
    reqs: Requirement[]
): array2d.Array2D<number> {
    const partsArr2d = array2d.full(-1, grid.cells.nrows, grid.cells.ncols);
    for (let y = 0; y < grid.cells.nrows; ++y) {
        for (let x = 0; x < grid.cells.ncols; ++x) {
            const v = grid.cells[y * partsArr2d.ncols + x];
            if (v < 0) {
                continue;
            }
            partsArr2d[y * partsArr2d.ncols + x] = reqs[v].partIndex;
        }
    }
    return partsArr2d;
}

function to8BitString(arr: number[]) {
    return String.fromCharCode(...arr.map((x) => x & 0xff));
}

export function* solve(
    parts: Part[],
    requirements: Requirement[],
    gridSettings: GridSettings,
    spinnableColors: boolean[]
): Iterable<Solution> {
    if (gridSettings.commandLineRow > gridSettings.height) {
        return;
    }

    // Very cheap check to see if this is even solvable at all.
    if (!requirementsAreAdmissible(parts, requirements, gridSettings)) {
        return;
    }

    const candidates = new Array<[number, Candidate[]]>(requirements.length);
    for (let i = 0; i < requirements.length; ++i) {
        const req = requirements[i];
        const part = parts[req.partIndex];
        candidates[i] = [
            i,
            candidatesForPart(
                part,
                gridSettings,
                req.constraint,
                spinnableColors[part.color] || false
            ),
        ];
    }

    // Heuristic: fit hard to fit blocks first, then easier ones.
    //
    // If two blocks are just as hard to fit, make sure to group ones of the same type together.
    candidates.sort(([i, a], [j, b]) => {
        const cmp = a.length - b.length;
        if (cmp != 0) {
            return cmp;
        }
        return i - j;
    });

    const visited = new Set();

    for (const solution of (function* helper(
        grid: Grid,
        candidateIdx: number
    ): Iterable<[number, Placement][]> {
        if (candidateIdx === candidates.length) {
            yield [];
            return;
        }

        const [reqIdx, cands] = candidates[candidateIdx];
        const req = requirements[reqIdx];
        const part = parts[req.partIndex];

        for (const candidate of cands) {
            const grid2 = grid.clone();
            if (
                !grid2.place(
                    candidate.mask,
                    candidate.placement.loc.position,
                    reqIdx
                )
            ) {
                continue;
            }

            if (
                !placementIsAdmissible(
                    grid2,
                    part.isSolid,
                    reqIdx,
                    req.constraint.onCommandLine,
                    req.constraint.bugged
                )
            ) {
                continue;
            }

            const gridByParts = to8BitString(
                partsArr2DForGrid(grid2, requirements)
            );
            if (visited.has(gridByParts)) {
                continue;
            }
            visited.add(gridByParts);

            for (const solution of helper(grid2, candidateIdx + 1)) {
                solution.push([reqIdx, candidate.placement]);
                if (
                    candidateIdx === candidates.length - 1 &&
                    !solutionIsAdmissible(parts, requirements, grid2)
                ) {
                    continue;
                }
                yield solution;
            }
        }
    })(new Grid(gridSettings), 0)) {
        solution.sort(([i, _1], [j, _2]) => i - j);
        yield solution.map(([_, p]) => p);
    }
}

function requirementsAreAdmissible(
    parts: Part[],
    requirements: Requirement[],
    gridSettings: GridSettings
) {
    // Mandatory check: blocks required to be on the command line must be less than or equal to the number of columns.
    let commandLineParts = 0;
    for (const req of requirements) {
        if (req.constraint.onCommandLine) {
            ++commandLineParts;
        }
    }
    if (commandLineParts > gridSettings.width) {
        return false;
    }

    // Mandatory check: total number of squares must be less than the total allowed space.
    let occupiedSquares = 0;
    for (const req of requirements) {
        const part = parts[req.partIndex];
        if (req.constraint.compressed === false) {
            occupiedSquares += arrayCountTrue(part.uncompressedMask);
        } else {
            occupiedSquares += arrayCountTrue(part.compressedMask);
        }
    }
    let availableSquares = gridSettings.width * gridSettings.height;
    if (gridSettings.hasOob) {
        availableSquares -= 4;
    }
    if (occupiedSquares > availableSquares) {
        return false;
    }

    return true;
}

function solutionIsAdmissible(
    parts: Part[],
    requirements: Requirement[],
    grid: Grid
) {
    interface PlacementDetail {
        outOfBounds: boolean;
        onCommandLine: boolean;
        touchingSameColor: boolean;
    }

    const placementDetails: PlacementDetail[] = new Array(requirements.length)
        .fill(null)
        .map((_) => ({
            outOfBounds: false,
            onCommandLine: false,
            touchingSameColor: false,
        }));

    for (let y = 0; y < grid.cells.nrows; ++y) {
        for (let x = 0; x < grid.cells.ncols; ++x) {
            const reqIdx = grid.cells[y * grid.cells.ncols + x];
            if (reqIdx < 0) {
                continue;
            }
            const req = requirements[reqIdx];
            const part = parts[req.partIndex];

            const placementDetail = placementDetails[reqIdx];

            // Optional admissibility: check if a block has/doesn't have any out of bounds parts.
            if (
                grid.hasOob &&
                (x === 0 ||
                    x === grid.cells.ncols - 1 ||
                    y === grid.cells.nrows - 1 ||
                    x === grid.cells.ncols - 1)
            ) {
                placementDetail.outOfBounds = true;
            }

            // Optional admissibility: check if a block is/isn't on the command line.
            if (y === grid.commandLineRow) {
                placementDetail.onCommandLine = true;
            }

            // Optional admissibility: check if same-colored blocks are appropriately touching/not touching.
            for (const [x2, y2] of [
                [x - 1, y],
                [x + 1, y],
                [x, y - 1],
                [x, y + 1],
            ]) {
                if (
                    x2 < 0 ||
                    x2 >= grid.cells.ncols ||
                    y2 < 0 ||
                    y2 >= grid.cells.nrows
                ) {
                    continue;
                }

                const neigborReqIdx = grid.cells[y2 * grid.cells.ncols + x2];
                if (neigborReqIdx < 0) {
                    continue;
                }

                const neigborReq = requirements[neigborReqIdx];
                const neighborPart = parts[neigborReq.partIndex];

                if (
                    neigborReqIdx != reqIdx &&
                    neighborPart.color === part.color
                ) {
                    placementDetail.touchingSameColor = true;
                    break;
                }
            }
        }
    }

    for (let i = 0; i < requirements.length; ++i) {
        const placementDetail = placementDetails[i];
        const req = requirements[i];
        const part = parts[req.partIndex];

        const placementIsBugged =
            placementDetail.outOfBounds ||
            part.isSolid === !placementDetail.onCommandLine ||
            placementDetail.touchingSameColor;

        if (
            req.constraint.bugged != null &&
            req.constraint.bugged != placementIsBugged
        ) {
            return false;
        }
    }

    return true;
}

function placementIsAdmissible(
    grid: Grid,
    isSolid: boolean,
    reqIdx: number,
    onCommandLine: boolean | null,
    bugged: boolean | null | null
) {
    // Mandatory admissibility: ensure not everything is out of bounds.
    if (grid.hasOob) {
        let isAllOob = true;
        top: for (let y = 1; y < grid.cells.nrows - 1; ++y) {
            for (let x = 1; x < grid.cells.ncols - 1; ++x) {
                const cell = grid.cells[y * grid.cells.ncols + x];
                if (cell === reqIdx) {
                    isAllOob = false;
                    break top;
                }
            }
        }
        if (isAllOob) {
            return false;
        }
    }

    // Optional admissibility: check if the block is appropriately in/out of bounds.
    let outOfBounds = false;
    if (grid.hasOob) {
        outOfBounds =
            arrayCountNumber(array2d.row(grid.cells, 0), reqIdx) > 0 ||
            arrayCountNumber(array2d.col(grid.cells, 0), reqIdx) > 0 ||
            arrayCountNumber(
                array2d.row(grid.cells, grid.cells.nrows - 1),
                reqIdx
            ) > 0 ||
            arrayCountNumber(
                array2d.col(grid.cells, grid.cells.ncols - 1),
                reqIdx
            ) > 0;
    }

    // Optional admissibility: check if the block is appropriately on/off the command line.
    const placedOnCommandLine =
        arrayCountNumber(array2d.row(grid.cells, grid.commandLineRow), reqIdx) >
        0;

    if (onCommandLine && !placedOnCommandLine) {
        return false;
    }

    // It is not possible to know if a piece is definitively not bugged, as it must pass the coloring check later also.
    const placementIsBugged = outOfBounds || isSolid === !placedOnCommandLine;
    if (bugged === false && placementIsBugged) {
        return false;
    }

    return true;
}

function candidatesForPart(
    part: Part,
    gridSettings: GridSettings,
    constraint: Constraint,
    spinnable: boolean
): Candidate[] {
    const candidates = [];
    if (
        constraint.compressed === true ||
        array2d.equal(part.compressedMask, part.uncompressedMask)
    ) {
        for (const { loc, mask } of placementLocationsAndMasksForMask(
            part.compressedMask,
            part.isSolid,
            gridSettings,
            constraint.onCommandLine,
            constraint.bugged,
            spinnable
        )) {
            candidates.push({ placement: { loc, compressed: true }, mask });
        }
    } else if (constraint.compressed === false) {
        for (const { loc, mask } of placementLocationsAndMasksForMask(
            part.uncompressedMask,
            part.isSolid,
            gridSettings,
            constraint.onCommandLine,
            constraint.bugged,
            spinnable
        )) {
            candidates.push({ placement: { loc, compressed: false }, mask });
        }
    } else {
        for (const { loc, mask } of placementLocationsAndMasksForMask(
            part.compressedMask,
            part.isSolid,
            gridSettings,
            constraint.onCommandLine,
            constraint.bugged,
            spinnable
        )) {
            candidates.push({ placement: { loc, compressed: true }, mask });
        }
        for (const { loc, mask } of placementLocationsAndMasksForMask(
            part.compressedMask,
            part.isSolid,
            gridSettings,
            constraint.onCommandLine,
            constraint.bugged,
            spinnable
        )) {
            candidates.push({ placement: { loc, compressed: false }, mask });
        }
    }
    return candidates;
}

function placementLocationsAndMasksForMask(
    mask: array2d.Array2D<boolean>,
    isSolid: boolean,
    gridSettings: GridSettings,
    onCommandLine: boolean | null,
    bugged: boolean | null,
    spinnable: boolean
) {
    const locations: { loc: Location; mask: array2d.Array2D<boolean> }[] = [];
    for (const position of placementPositionsForMask(
        mask,
        isSolid,
        gridSettings,
        onCommandLine,
        bugged
    )) {
        locations.push({ loc: { position, rotation: 0 }, mask });
    }

    if (spinnable) {
        const knownMasks = new Set();
        knownMasks.add(to8BitString(arrayBooleanToNumber(trim(mask))));

        for (let i = 1; i < 4; ++i) {
            const mask2 = array2d.rot90(mask);
            const knownMask2 = to8BitString(arrayBooleanToNumber(trim(mask2)));
            if (knownMasks.has(knownMask2)) {
                continue;
            }
            knownMasks.add(knownMask2);

            for (const position of placementPositionsForMask(
                mask2,
                isSolid,
                gridSettings,
                onCommandLine,
                bugged
            )) {
                locations.push({
                    loc: { position, rotation: i },
                    mask: mask2,
                });
            }
        }
    }

    return locations;
}
function placementPositionsForMask(
    mask: array2d.Array2D<boolean>,
    isSolid: boolean,
    gridSettings: GridSettings,
    onCommandLine: boolean | null,
    bugged: boolean | null
) {
    const positions = [];

    for (let y = 0; y < mask.nrows; ++y) {
        for (let x = 0; x < mask.ncols; ++x) {
            const pos = { x, y };
            const grid = new Grid(gridSettings);
            if (!grid.place(mask, pos, 0)) {
                continue;
            }

            if (
                !placementIsAdmissible(grid, isSolid, 0, onCommandLine, bugged)
            ) {
                continue;
            }

            positions.push(pos);
        }
    }

    return positions;
}
