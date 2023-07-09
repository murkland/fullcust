export interface Array2D<T> extends Array<T> {
    nrows: number;
    ncols: number;
}

function Array2D<T>(nrows: number, ncols: number): Array2D<T> {
    const arr2d = new Array(nrows * ncols) as Array2D<T>;
    arr2d.nrows = nrows;
    arr2d.ncols = ncols;
    return arr2d;
}

export function from<T>(data: T[], nrows: number, ncols: number): Array2D<T> {
    const arr2d = [...data] as Array2D<T>;
    arr2d.nrows = nrows;
    arr2d.ncols = ncols;
    return arr2d;
}

export function full<T>(v: T, nrows: number, ncols: number): Array2D<T> {
    const arr2d = Array2D<T>(nrows, ncols);
    arr2d.fill(v, 0, nrows * ncols);
    return arr2d;
}

export function copy<T>(arr2d: Array2D<T>): Array2D<T> {
    return from(arr2d, arr2d.nrows, arr2d.ncols);
}

export function subarray<T>(
    arr2d: Array2D<T>,
    top: number,
    left: number,
    nrows: number,
    ncols: number
) {
    const subarr2d = Array2D<T>(nrows, ncols);
    for (let y = 0; y < nrows; ++y) {
        for (let x = 0; x < ncols; ++x) {
            subarr2d[y * ncols + x] =
                arr2d[(top + y) * arr2d.ncols + (left + x)];
        }
    }
    return subarr2d;
}

export function transpose<T>(arr2d: Array2D<T>) {
    const transposed = Array2D<T>(arr2d.ncols, arr2d.nrows);
    for (let y = 0; y < arr2d.nrows; ++y) {
        for (let x = 0; x < arr2d.ncols; ++x) {
            transposed[x * transposed.ncols + y] = arr2d[y * arr2d.ncols + x];
        }
    }
    return transposed;
}

export function flipRowsInplace<T>(arr2d: Array2D<T>) {
    for (let y = 0; y < arr2d.nrows; ++y) {
        const limit = Math.floor(arr2d.ncols / 2);
        for (let x = 0; x < limit; ++x) {
            const tmp = arr2d[y * arr2d.ncols + x];
            arr2d[y * arr2d.ncols + x] =
                arr2d[y * arr2d.ncols + (arr2d.ncols - x) - 1];
            arr2d[y * arr2d.ncols + (arr2d.ncols - x) - 1] = tmp;
        }
    }
}

export function rot90<T>(arr2d: Array2D<T>) {
    const transposed = transpose(arr2d);
    flipRowsInplace(transposed);
    return transposed;
}

export function equal<T>(l: Array2D<T>, r: Array2D<T>) {
    if (l.nrows != r.nrows || l.ncols != r.ncols) {
        return false;
    }
    for (let i = 0; i < l.length; ++i) {
        if (l[i] != r[i]) {
            return false;
        }
    }
    return true;
}

export function pretty<T extends { toString: () => string }>(
    arr2d: Array2D<T>
): string {
    const buf: string[] = [];
    for (let y = 0; y < arr2d.nrows; ++y) {
        for (let x = 0; x < arr2d.ncols; ++x) {
            buf.push(arr2d[y * arr2d.ncols + x].toString());
            buf.push("\t");
        }
        buf.push("\n");
    }
    return buf.join("");
}

export function row<T>(arr2d: Array2D<T>, y: number): T[] {
    return arr2d.slice(y * arr2d.ncols, (y + 1) * arr2d.ncols);
}

export function col<T>(arr2d: Array2D<T>, x: number): T[] {
    const col = new Array<T>(arr2d.nrows);
    for (let y = 0; y < arr2d.nrows; ++y) {
        col[y] = arr2d[y * arr2d.ncols + x];
    }
    return col;
}

export default Array2D;
