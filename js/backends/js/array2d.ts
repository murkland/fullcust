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

export function from<T>(
    data: Array<T>,
    nrows: number,
    ncols: number
): Array2D<T> {
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
    for (let i = 0; i < nrows; ++i) {
        for (let j = 0; j < ncols; ++j) {
            subarr2d[i * ncols + j] =
                arr2d[(top + i) * arr2d.ncols + (left + j)];
        }
    }
    return subarr2d;
}

export function transpose<T>(arr2d: Array2D<T>) {
    const transposed = Array2D<T>(arr2d.ncols, arr2d.nrows);
    for (let i = 0; i < arr2d.nrows; ++i) {
        for (let j = 0; j < arr2d.ncols; ++j) {
            transposed[j * transposed.ncols + i] = arr2d[i * arr2d.ncols + j];
        }
    }
    return transposed;
}

export function flipRowsInplace<T>(arr2d: Array2D<T>) {
    for (let i = 0; i < arr2d.nrows; ++i) {
        const limit = Math.floor(arr2d.ncols / 2);
        for (let j = 0; j < limit; ++j) {
            let tmp = arr2d[i * arr2d.ncols + j];
            arr2d[i * arr2d.ncols + j] =
                arr2d[i * arr2d.ncols + (arr2d.ncols - j) - 1];
            arr2d[i * arr2d.ncols + (arr2d.ncols - j) - 1] = tmp;
        }
    }
}

export function rot90<T>(arr2d: Array2D<T>) {
    const transposed = transpose(arr2d);
    flipRowsInplace(transposed);
    return transposed;
}

export function equal<T>(l: Array2D<T>, r: Array2D<T>) {
    return (
        l.nrows == r.nrows && l.ncols == r.ncols && l.every((v, i) => v == r[i])
    );
}

export function pretty<T>(arr2d: Array2D<T>): string {
    const buf = [];
    for (let i = 0; i < arr2d.nrows; ++i) {
        for (let j = 0; j < arr2d.ncols; ++j) {
            buf.push(arr2d[i * arr2d.ncols + j]);
            buf.push("\t");
        }
        buf.push("\n");
    }
    return buf.join("");
}

export function row<T>(arr2d: Array2D<T>, i: number): T[] {
    return arr2d.slice(i * arr2d.ncols, (i + 1) * arr2d.ncols);
}

export function col<T>(arr2d: Array2D<T>, j: number): T[] {
    const col = new Array<T>(arr2d.nrows);
    for (let i = 0; i < arr2d.nrows; ++i) {
        col[i] = arr2d[i * arr2d.ncols + j];
    }
    return col;
}

export default Array2D;
