import { solve } from "./solver";

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
                height: 7,
                width: 7,
                hasOob: false,
                commandLineRow: 1,
            }
        )
    )
);
