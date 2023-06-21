import isEqual from "lodash-es/isEqual";
import React, { ChangeEvent } from "react";
import { createRoot } from "react-dom/client";
import { useInView } from "react-intersection-observer";

import AsyncSolver from "./async-solver";
import { convertParts, GridSettings, Part, placeAll, Requirement, Solution } from "./solver";

const queryParams = new URLSearchParams(location.search);
const game = queryParams.get("game") || "bn6";

const COLORS = {
    red: {
        name: "red",
        nameJa: "レッド",
        solid: "#de1000",
        plus: "#bd0000",
    },
    pink: {
        name: "pink",
        nameJa: "ピンク",
        solid: "#de8cc6",
        plus: "#bd6ba5",
    },
    yellow: {
        name: "yellow",
        nameJa: "イエロー",
        solid: "#dede00",
        plus: "#bdbd00",
    },
    green: {
        name: "green",
        nameJa: "グリーン",
        solid: "#18c600",
        plus: "#00a500",
    },
    blue: {
        name: "blue",
        nameJa: "ブルー",
        solid: "#2984de",
        plus: "#0860b8",
    },
    white: {
        name: "white",
        nameJa: "ホワイト",
        solid: "#dedede",
        plus: "#bdbdbd",
    },
    orange: {
        name: "orange",
        nameJa: "オレンジ",
        solid: "#de7b00",
        plus: "#bd5a00",
    },
    purple: {
        name: "purple",
        nameJa: "パープル",
        solid: "#9400ce",
        plus: "#7300ad",
    },
    gray: {
        name: "gray",
        nameJa: "グレー",
        solid: "#848484",
        plus: "#636363",
    },
};

function fromHashString(s: string): Problem | null {
    if (s == "") {
        return null;
    }

    const j = JSON.parse(s) as {
        r: {
            i: number;
            c: number;
            b: number;
            z: number;
        }[];
        s: number[];
        x: number;
    };
    return {
        requirements: j.r.map((cr) => ({
            partIndex: cr.i,
            constraint: {
                onCommandLine: cr.c === 1 ? true : cr.c === 0 ? false : null,
                bugged: cr.b === 1 ? true : cr.b === 0 ? false : null,
                compressed: cr.z === 1 ? true : cr.z === 0 ? false : null,
            },
        })),
        spinnableColors: j.s.map((v) => !!v),
        expansionMemories: j.x,
    };
}

function toHashString(problem: Problem): string {
    return JSON.stringify({
        r: problem.requirements.map((req) => ({
            i: req.partIndex,
            c:
                req.constraint.onCommandLine === true
                    ? 1
                    : req.constraint.onCommandLine === false
                    ? 0
                    : -1,
            b:
                req.constraint.bugged === true
                    ? 1
                    : req.constraint.bugged === false
                    ? 0
                    : -1,
            z:
                req.constraint.compressed === true
                    ? 1
                    : req.constraint.compressed === false
                    ? 0
                    : -1,
        })),
        s: problem.spinnableColors.map((v) => (v ? 1 : 0)),
        x: problem.expansionMemories,
    });
}

interface Data {
    colors: string[];
    gridSettings: GridSettings;
    parts: Part[];
    partMetas: {
        name: string;
        nameJa: string;
        compressedMask: (0 | 1)[];
        uncompressedMask: (0 | 1)[];
    }[];
}

interface Problem {
    requirements: Requirement[];
    spinnableColors: boolean[];
    expansionMemories: number;
}

function emptyProblem(data: Data): Problem {
    return {
        requirements: [],
        spinnableColors: data.colors.map((_: string) => true),
        expansionMemories: 2,
    };
}

function ConstraintDropdown({
    title,
    value,
    onChange,
    disabled = false,
}: {
    title: string;
    value: boolean | null;
    onChange: (value: boolean | null) => void;
    disabled?: boolean;
}) {
    return (
        <div className="form-floating">
            <select
                disabled={disabled}
                value={JSON.stringify(value)}
                className="form-select"
                onChange={(e) => {
                    onChange(JSON.parse(e.target.value));
                }}
            >
                {[
                    [null, "🤷 maybe・任意"],
                    [false, "❌ must not・不要"],
                    [true, "✅ must・必要"],
                ].map(([v, label]) => {
                    return (
                        <option
                            value={JSON.stringify(v)}
                            key={JSON.stringify(v)}
                        >
                            {label}
                        </option>
                    );
                })}
            </select>
            <label>{title}</label>
        </div>
    );
}

function PartSelector({
    data,
    problem,
    onChange,
}: {
    data: Data;
    problem: Problem;
    onChange: (problem: Problem) => void;
}) {
    return (
        <>
            <div className="form mb-2">
                <div className="col">
                    <select
                        className="form-select"
                        id="part-select"
                        disabled={data == null}
                        value={""}
                        onChange={(e) => {
                            const partIndex = parseInt(e.target.value, 10);
                            const part = data!.parts[partIndex];

                            onChange({
                                ...problem,
                                requirements: [
                                    {
                                        partIndex,
                                        constraint: {
                                            bugged: null,
                                            compressed: !isEqual(
                                                part.compressedMask,
                                                part.uncompressedMask
                                            )
                                                ? true
                                                : false,
                                            onCommandLine: part.isSolid
                                                ? true
                                                : null,
                                        },
                                    },
                                    ...problem.requirements,
                                ],
                            });
                        }}
                    >
                        <option value="" disabled>
                            Pick a part・パートを選択
                        </option>
                        {data != null
                            ? data.partMetas.map((part, i) => (
                                  <option value={i} key={i}>
                                      {part.name}・{part.nameJa}
                                  </option>
                              ))
                            : null}
                    </select>
                </div>
            </div>

            <div className="overflow-auto flex-grow-1 mb-2">
                <ol className="list-group">
                    {problem.requirements.map((requirement, i) => {
                        const partMeta = data!.partMetas[requirement.partIndex];

                        return (
                            <li className="list-group-item" key={i}>
                                <div className="mb-2 d-flex align-items-center">
                                    <div className="flex-grow-1">
                                        {i + 1}. {partMeta.name}・
                                        {partMeta.nameJa}
                                    </div>
                                    <button
                                        type="button"
                                        className="btn btn-close btn-sm align-self-end"
                                        onClick={((i: number) => {
                                            onChange({
                                                ...problem,
                                                requirements:
                                                    problem.requirements.filter(
                                                        (_, j) => i != j
                                                    ),
                                            });
                                        }).bind(null, i)}
                                    />
                                </div>
                                <div className="row g-2">
                                    <div className="col-xl">
                                        <ConstraintDropdown
                                            value={
                                                requirement.constraint
                                                    .onCommandLine
                                            }
                                            title="on command line・コマンドライン上"
                                            onChange={((
                                                i: number,
                                                v: boolean | null
                                            ) => {
                                                onChange({
                                                    ...problem,
                                                    requirements:
                                                        problem.requirements.map(
                                                            (r, j) =>
                                                                i == j
                                                                    ? {
                                                                          ...r,
                                                                          constraint:
                                                                              {
                                                                                  ...r.constraint,
                                                                                  onCommandLine:
                                                                                      v,
                                                                              },
                                                                      }
                                                                    : r
                                                        ),
                                                });
                                            }).bind(null, i)}
                                        />
                                    </div>
                                    <div className="col-xl">
                                        <ConstraintDropdown
                                            value={
                                                requirement.constraint.bugged
                                            }
                                            title="cause bug・バグを引き起こす"
                                            onChange={((
                                                i: number,
                                                v: boolean | null
                                            ) => {
                                                onChange({
                                                    ...problem,
                                                    requirements:
                                                        problem.requirements.map(
                                                            (r, j) =>
                                                                i == j
                                                                    ? {
                                                                          ...r,
                                                                          constraint:
                                                                              {
                                                                                  ...r.constraint,
                                                                                  bugged: v,
                                                                              },
                                                                      }
                                                                    : r
                                                        ),
                                                });
                                            }).bind(null, i)}
                                        />
                                    </div>
                                    <div className="col-xl">
                                        <ConstraintDropdown
                                            value={
                                                requirement.constraint
                                                    .compressed
                                            }
                                            title="compress・圧縮"
                                            disabled={isEqual(
                                                partMeta.compressedMask,
                                                partMeta.uncompressedMask
                                            )}
                                            onChange={((
                                                i: number,
                                                v: boolean | null
                                            ) => {
                                                onChange({
                                                    ...problem,
                                                    requirements:
                                                        problem.requirements.map(
                                                            (r, j) =>
                                                                i == j
                                                                    ? {
                                                                          ...r,
                                                                          constraint:
                                                                              {
                                                                                  ...r.constraint,
                                                                                  compressed:
                                                                                      v,
                                                                              },
                                                                      }
                                                                    : r
                                                        ),
                                                });
                                            }).bind(null, i)}
                                        />
                                    </div>
                                </div>
                            </li>
                        );
                    })}
                </ol>
            </div>

            <div className="accordion mb-2" id="extra-settings">
                <div className="accordion-item">
                    <h2 className="accordion-header" id="extra-settings-header">
                        <button
                            className="accordion-button collapsed"
                            type="button"
                            data-bs-toggle="collapse"
                            data-bs-target="#extra-settings-body"
                            aria-expanded="false"
                            aria-controls="extra-settings-body"
                        >
                            Extra settings・追加設定
                        </button>
                    </h2>
                    <div
                        id="extra-settings-body"
                        className="accordion-collapse collapse"
                        aria-labelledby="extra-settings-header"
                        data-bs-parent="#extra-settings"
                    >
                        <div className="accordion-body">
                            <div className="row mb-2">
                                <label
                                    htmlFor="expansion-memories"
                                    className="col-sm-4 col-form-label"
                                >
                                    Expansion memory・拡張メモリ
                                </label>
                                <div className="col-sm-8">
                                    <select
                                        className="form-select form-select-small"
                                        id="expansion-memories"
                                        value={problem.expansionMemories}
                                        onChange={(e) => {
                                            onChange({
                                                ...problem,
                                                expansionMemories: parseInt(
                                                    e.target.value,
                                                    10
                                                ),
                                            });
                                        }}
                                    >
                                        <option value={0}>4×4</option>
                                        <option value={1}>5×4</option>
                                        <option value={2}>5×5</option>
                                    </select>
                                </div>
                            </div>

                            <div className="mb-2">
                                {data.colors.map((color, i) => {
                                    if (i == 0) {
                                        return null;
                                    }

                                    return (
                                        <div
                                            className="form-check form-check-inline"
                                            key={i}
                                        >
                                            <input
                                                type="checkbox"
                                                className="form-check-input"
                                                id={`spinnable-${color}`}
                                                onChange={((
                                                    i: number,
                                                    e: ChangeEvent<HTMLInputElement>
                                                ) => {
                                                    onChange({
                                                        ...problem,
                                                        spinnableColors:
                                                            problem.spinnableColors.map(
                                                                (v, j) =>
                                                                    i == j
                                                                        ? e
                                                                              .target
                                                                              .checked
                                                                        : v
                                                            ),
                                                    });
                                                }).bind(null, i)}
                                                checked={
                                                    problem.spinnableColors[i]
                                                }
                                            />
                                            <label
                                                htmlFor={`spinnable-${color}`}
                                                className="form-check-label px-2 rounded"
                                                style={{
                                                    backgroundColor:
                                                        COLORS[
                                                            color as keyof typeof COLORS
                                                        ].solid,
                                                }}
                                            >
                                                spin{" "}
                                                {
                                                    COLORS[
                                                        color as keyof typeof COLORS
                                                    ].name
                                                }
                                                ・スピン
                                                {
                                                    COLORS[
                                                        color as keyof typeof COLORS
                                                    ].nameJa
                                                }
                                            </label>
                                        </div>
                                    ); // TODO: Spinnables
                                })}
                            </div>
                        </div>
                    </div>
                </div>
            </div>

            <div className="row">
                <div className="col d-flex justify-content-end">
                    <button
                        type="reset"
                        id="reset"
                        className="btn btn-danger"
                        onClick={() => {
                            onChange(emptyProblem(data));
                        }}
                    >
                        <span>Reset・リセット</span>
                    </button>
                </div>
            </div>
        </>
    );
}

const CELL_SIZE = 96;

const BORDER_WIDTH = 8;
const BG_FILL_COLOR = "#202020";
const BORDER_STROKE_COLOR = "#000000";

function drawGridView(
    ctx: CanvasRenderingContext2D,
    parts: Part[],
    colors: string[],
    requirements: Requirement[],
    cells: (number | undefined)[],
    gridSettings: GridSettings
) {
    ctx.lineWidth = BORDER_WIDTH;
    ctx.font = `${20 * 2}px sans-serif`;
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";

    // First pass: draw background.
    ctx.strokeStyle = BORDER_STROKE_COLOR;
    ctx.fillStyle = BG_FILL_COLOR;
    for (let y = 0; y < gridSettings.height; ++y) {
        for (let x = 0; x < gridSettings.width; ++x) {
            const px = x * CELL_SIZE + BORDER_WIDTH / 2;
            const py = y * CELL_SIZE + BORDER_WIDTH / 2;

            if (
                gridSettings.hasOob &&
                ((x == 0 && y == 0) ||
                    (x == 0 && y == gridSettings.height - 1) ||
                    (x == gridSettings.width - 1 && y == 0) ||
                    (x == gridSettings.width - 1 &&
                        y == gridSettings.height - 1))
            ) {
                continue;
            }

            ctx.fillRect(px, py, CELL_SIZE, CELL_SIZE);

            // top
            ctx.strokeRect(px, py, CELL_SIZE, 1);

            // bottom
            ctx.strokeRect(px, py + CELL_SIZE, CELL_SIZE, 1);

            // left
            ctx.strokeRect(px, py, 1, CELL_SIZE);

            // right
            ctx.strokeRect(px + CELL_SIZE, py, 1, CELL_SIZE);
        }
    }

    // Second pass: draw squares.
    for (let y = 0; y < gridSettings.height; ++y) {
        for (let x = 0; x < gridSettings.width; ++x) {
            const cell = cells[y * gridSettings.width + x];
            if (cell == null) {
                continue;
            }

            const requirement = requirements[cell];
            const part = parts[requirement.partIndex];
            const color = COLORS[colors[part.color] as keyof typeof COLORS];

            const px = x * CELL_SIZE + BORDER_WIDTH / 2;
            const py = y * CELL_SIZE + BORDER_WIDTH / 2;

            ctx.fillStyle = color.solid;
            ctx.strokeStyle = color.plus;

            ctx.fillRect(px, py, CELL_SIZE, CELL_SIZE);

            ctx.strokeRect(px, py, CELL_SIZE, 1);
            ctx.strokeRect(px, py + CELL_SIZE, CELL_SIZE, 1);
            ctx.strokeRect(px, py, 1, CELL_SIZE);
            ctx.strokeRect(px + CELL_SIZE, py, 1, CELL_SIZE);
            if (!part.isSolid) {
                ctx.strokeRect(px, py + CELL_SIZE / 2, CELL_SIZE, 1);
                ctx.strokeRect(px + CELL_SIZE / 2, py, 1, CELL_SIZE);
            }

            ctx.fillStyle = BORDER_STROKE_COLOR;
            ctx.fillText(
                (cell + 1).toString(),
                px + CELL_SIZE / 2,
                py + CELL_SIZE / 2
            );
        }
    }

    // Third pass: draw borders.
    ctx.strokeStyle = BORDER_STROKE_COLOR;

    for (let y = 0; y < gridSettings.height; ++y) {
        for (let x = 0; x < gridSettings.width; ++x) {
            const cell = cells[y * gridSettings.width + x];
            if (cell == null) {
                continue;
            }

            const px = x * CELL_SIZE + BORDER_WIDTH / 2;
            const py = y * CELL_SIZE + BORDER_WIDTH / 2;

            // top
            if (y == 0 || cells[(y - 1) * gridSettings.width + x] != cell) {
                ctx.strokeRect(px, py, CELL_SIZE, 1);
            }

            // bottom
            if (
                y == gridSettings.height - 1 ||
                cells[(y + 1) * gridSettings.width + x] != cell
            ) {
                ctx.strokeRect(px, py + CELL_SIZE, CELL_SIZE, 1);
            }

            // left
            if (x == 0 || cells[y * gridSettings.width + (x - 1)] != cell) {
                ctx.strokeRect(px, py, 1, CELL_SIZE);
            }

            // right
            if (
                x == gridSettings.width - 1 ||
                cells[y * gridSettings.width + (x + 1)] != cell
            ) {
                ctx.strokeRect(px + CELL_SIZE, py, 1, CELL_SIZE);
            }
        }
    }

    // Fourth pass: draw command line.
    const commandLinePy =
        gridSettings.commandLineRow * CELL_SIZE + BORDER_WIDTH / 2;
    ctx.strokeRect(
        0,
        commandLinePy + (CELL_SIZE * 1.0) / 4.0,
        gridSettings.width * CELL_SIZE + BORDER_WIDTH,
        1
    );
    ctx.strokeRect(
        0,
        commandLinePy + (CELL_SIZE * 3.0) / 4.0,
        gridSettings.width * CELL_SIZE + BORDER_WIDTH,
        1
    );

    // Fifth pass: draw out of bounds overlay.
    if (gridSettings.hasOob) {
        ctx.fillStyle = "rgba(0, 0, 0, 0.5)";
        ctx.beginPath();
        ctx.rect(
            CELL_SIZE,
            0,
            (gridSettings.width - 2) * CELL_SIZE + BORDER_WIDTH,
            CELL_SIZE + BORDER_WIDTH * 2 - BORDER_WIDTH
        );
        ctx.rect(
            CELL_SIZE,
            gridSettings.height * CELL_SIZE - CELL_SIZE,
            (gridSettings.width - 2) * CELL_SIZE + BORDER_WIDTH,
            CELL_SIZE + BORDER_WIDTH * 2 - BORDER_WIDTH
        );
        ctx.rect(
            gridSettings.width * CELL_SIZE - CELL_SIZE,
            CELL_SIZE,
            CELL_SIZE + BORDER_WIDTH * 2 - BORDER_WIDTH,
            (gridSettings.height - 2) * CELL_SIZE + BORDER_WIDTH
        );
        ctx.rect(
            0,
            CELL_SIZE,
            CELL_SIZE + BORDER_WIDTH * 2 - BORDER_WIDTH,
            (gridSettings.height - 2) * CELL_SIZE + BORDER_WIDTH
        );
        ctx.closePath();
        ctx.fill();
    }
}

function Navicust({
    parts,
    colors,
    requirements,
    cells,
    gridSettings,
}: {
    parts: Part[];
    colors: string[];
    requirements: Requirement[];
    cells: (number | undefined)[];
    gridSettings: GridSettings;
}) {
    const canvasRef = React.useRef<HTMLCanvasElement | null>(null);
    React.useEffect(() => {
        const ctx = canvasRef.current!.getContext("2d")!;
        drawGridView(ctx, parts, colors, requirements, cells, gridSettings);
    }, []);

    return (
        <canvas
            ref={canvasRef}
            width={gridSettings.width * CELL_SIZE + BORDER_WIDTH}
            height={gridSettings.height * CELL_SIZE + BORDER_WIDTH}
            style={{
                width: (gridSettings.width * CELL_SIZE + BORDER_WIDTH) / 2,
                height: (gridSettings.height * CELL_SIZE + BORDER_WIDTH) / 2,
            }}
        />
    );
}

const NavicustPlaceholder = React.forwardRef(
    (
        { gridSettings }: { gridSettings: GridSettings },
        ref: React.Ref<HTMLDivElement>
    ) => {
        return (
            <div
                ref={ref}
                className="d-flex justify-content-center align-items-center"
                style={{
                    width: (gridSettings.width * CELL_SIZE + BORDER_WIDTH) / 2,
                    height:
                        (gridSettings.height * CELL_SIZE + BORDER_WIDTH) / 2,
                }}
            >
                <div className="spinner-border" />
            </div>
        );
    }
);

function makeGridSettings(data: Data, problem: Problem): GridSettings {
    return {
        ...data.gridSettings,
        height:
            data.gridSettings.height - (problem.expansionMemories < 2 ? 1 : 0),
        width:
            data.gridSettings.width - (problem.expansionMemories < 1 ? 1 : 0),
    };
}

function Results({ problem, data }: { problem: Problem; data: Data }) {
    const gs = makeGridSettings(data, problem);

    const [pending, setPending] = React.useState(false);
    const [done, setDone] = React.useState(false);
    const [solutions, setSolutions] = React.useState<Solution[]>([]);
    const solverRef = React.useRef<AsyncSolver | null>(null);
    if (solverRef.current == null) {
        solverRef.current = new AsyncSolver(
            data.parts,
            problem.requirements,
            gs,
            problem.spinnableColors
        );
    }

    const { ref, inView } = useInView({});

    React.useEffect(() => {
        if (problem.requirements.length == 0) {
            setDone(true);
            return;
        }

        const solver = new AsyncSolver(
            data.parts,
            problem.requirements,
            gs,
            problem.spinnableColors
        );

        solverRef.current = solver;

        return () => {
            solver.kill();
        };
    }, [problem, data, setSolutions, setDone]);

    React.useEffect(() => {
        (async () => {
            if (solverRef.current == null || done || pending || !inView) {
                return;
            }
            setPending(true);
            let { done: nextDone, value } = await solverRef.current.next();
            if (nextDone) {
                setDone(true);
            } else {
                setSolutions((solutions) => [...solutions, value]);
            }
            setPending(false);
        })();
    }, [inView, done, pending, setDone, setSolutions]);

    return (
        <div
            style={{
                display: "grid",
                gridTemplateColumns: "repeat(auto-fit, minmax(350px, 1fr))",
                gap: "1rem",
            }}
        >
            {solutions.length > 0 ? (
                solutions.map((solution, i) => (
                    <div style={{ textAlign: "center" }} key={i}>
                        <Navicust
                            parts={data.parts}
                            colors={data.colors}
                            requirements={problem.requirements}
                            gridSettings={gs}
                            cells={placeAll(
                                data.parts,
                                problem.requirements,
                                solution as Solution,
                                gs
                            )}
                        />
                    </div>
                ))
            ) : done ? (
                problem.requirements.length > 0 ? (
                    <div key="no-results" className="alert alert-danger">
                        No solutions found・解決が発見されなかった
                    </div>
                ) : (
                    <div key="no-requirements" className="alert alert-info">
                        Select parts from the left to start
                        solving・解決を開始するために、左面からパートを選択ください
                    </div>
                )
            ) : null}
            {done ? null : <NavicustPlaceholder ref={ref} gridSettings={gs} />}
        </div>
    );
}

function App() {
    const [data, setData] = React.useState<Data | null>(null);
    const [problem, setProblem] = React.useState<Problem | null>(
        fromHashString(decodeURIComponent(window.location.hash.slice(1)))
    );

    React.useEffect(() => {
        (async () => {
            const raw = await import(`./data/${game}.json`);
            const data = {
                colors: raw.colors,
                gridSettings: raw.gridSettings,
                partMetas: raw.parts,
                parts: convertParts(
                    raw.parts,
                    raw.gridSettings.height,
                    raw.gridSettings.width
                ),
            };
            setProblem((problem) => {
                if (problem == null) {
                    problem = emptyProblem(data);
                    window.location.hash = toHashString(problem);
                    return problem;
                }
                return problem;
            });
            setData(data);
        })();
    }, [setProblem, setData]);

    return (
        <div>
            <div
                className="h-100 position-fixed d-flex flex-column p-2 border-end"
                style={{ width: "40%", top: 0 }}
            >
                <h1 className="h4">
                    fullcust{" "}
                    <small className="text-muted">
                        navicust autolayout・ナビカスタマイザー自動配置
                    </small>
                </h1>
                <ul id="games-nav" className="nav nav-pills mb-2">
                    {[
                        ["bn6", "bn6・exe6"],
                        ["bn5", "bn5・exe5"],
                        ["bn4", "bn4・exe4"],
                    ].map(([key, label]) => (
                        <li className="nav-item" key={key}>
                            <a
                                className={`nav-link ${
                                    game == key ? "active" : ""
                                }`}
                                href={`?game=${key}`}
                            >
                                {label}
                            </a>
                        </li>
                    ))}
                </ul>
                {data != null && problem != null ? (
                    <PartSelector
                        data={data}
                        problem={problem}
                        onChange={(problem) => {
                            window.location.hash = toHashString(problem);
                            setProblem(problem);
                        }}
                    />
                ) : null}
            </div>
            <div
                className="container-fluid my-2"
                style={{ marginLeft: "40%", width: "60%" }}
            >
                {data != null && problem != null ? (
                    <Results
                        key={JSON.stringify(problem)}
                        data={data}
                        problem={problem}
                    />
                ) : null}
            </div>
        </div>
    );
}

createRoot(document.getElementById("main")!).render(<App />);
