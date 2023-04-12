import isEqual from "lodash-es/isEqual";

import { convertParts, GridSettings, Part, placeAll, Requirement, Solution } from "./solver";

async function main() {
    let requirements: Requirement[] = [];

    const data = await import("./bn6.json");

    const parts = convertParts(data.parts, 7, 7);

    const partSelect = document.getElementById(
        "part-select"
    )! as HTMLSelectElement;
    partSelect.disabled = false;

    const results = document.getElementById("results")!;
    const noResults = document.getElementById("no-results");

    for (let i = 0; i < parts.length; ++i) {
        const part = parts[i];

        const option = document.createElement("option");
        partSelect.appendChild(option);
        option.value = i.toString();
        option.textContent = `${part.name}・${part.nameJa}`;
    }

    const requirementsEl = document.getElementById("requirements")!;

    partSelect.onchange = () => {
        const partIndex = parseInt(partSelect.value, 10);
        const part = parts[partIndex];

        requirements.push({
            partIndex,
            constraint: {
                bugged: null,
                compressed: true,
                onCommandLine: part.isSolid ? true : null,
            },
        });
        partSelect.value = "";
        update();
    };

    function createConstraintDropdown(
        title: string,
        initialValue: boolean | null,
        onchange: (v: boolean | null) => void
    ) {
        const el = document.createElement("div");
        el.className = "form-floating";

        const select = document.createElement("select");
        el.appendChild(select);

        for (const [v, text] of [
            [null, "🤷 maybe・任意"],
            [false, "❌ must not・不要"],
            [true, "✅ must・必要"],
        ] as [boolean | null, string][]) {
            const option = document.createElement("option");
            select.appendChild(option);
            option.value = JSON.stringify(v);
            option.textContent = text;
        }

        select.className = "form-select";
        select.onchange = () => {
            onchange(JSON.parse(select.value));
        };
        select.value = JSON.stringify(initialValue);

        const label = document.createElement("label");
        el.append(label);
        label.textContent = title;

        return el;
    }

    const gridSettings: GridSettings = {
        height: 7,
        width: 7,
        hasOob: true,
        commandLineRow: 3,
    };

    const CELL_SIZE = 48;

    const COLORS = {
        red: {
            solid: "#de1000",
            plus: "#bd0000",
        },
        pink: {
            solid: "#de8cc6",
            plus: "#bd6ba5",
        },
        yellow: {
            solid: "#dede00",
            plus: "#bdbd00",
        },
        green: {
            solid: "#18c600",
            plus: "#00a500",
        },
        blue: {
            solid: "#2984de",
            plus: "#0860b8",
        },
        white: {
            solid: "#dedede",
            plus: "#bdbdbd",
        },
        orange: {
            solid: "#de7b00",
            plus: "#bd5a00",
        },
        purple: {
            solid: "#9400ce",
            plus: "#7300ad",
        },
        gray: {
            solid: "#848484",
            plus: "#636363",
        },
    };

    const BORDER_WIDTH = 4;
    const BG_FILL_COLOR = "#202020";
    const BORDER_STROKE_COLOR = "#000000";

    function drawGridView(
        ctx: CanvasRenderingContext2D,
        parts: Part[],
        requirements: Requirement[],
        cells: (number | null)[],
        gridSettings: GridSettings
    ) {
        ctx.lineWidth = BORDER_WIDTH;
        ctx.font = "20px sans-serif";
        ctx.textAlign = "center";
        ctx.textBaseline = "middle";

        // First pass: draw background.
        ctx.strokeStyle = BORDER_STROKE_COLOR;
        for (let y = 0; y < gridSettings.height; ++y) {
            for (let x = 0; x < gridSettings.width; ++x) {
                const px = x * CELL_SIZE + BORDER_WIDTH / 2;
                const py = y * CELL_SIZE + BORDER_WIDTH / 2;

                ctx.fillStyle = BG_FILL_COLOR;
                if (
                    gridSettings.hasOob &&
                    ((x == 0 && y == 0) ||
                        (x == 0 && y == gridSettings.height - 1) ||
                        (x == gridSettings.width - 1 && y == 0) ||
                        (x == gridSettings.width - 1 &&
                            y == gridSettings.height - 1))
                ) {
                    ctx.fillStyle = BORDER_STROKE_COLOR;
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
                const color =
                    COLORS[data.colors[part.color] as keyof typeof COLORS];

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
                0,
                0,
                gridSettings.width * CELL_SIZE + BORDER_WIDTH,
                CELL_SIZE + BORDER_WIDTH * 2 - BORDER_WIDTH / 2
            );
            ctx.rect(
                0,
                gridSettings.height * CELL_SIZE - CELL_SIZE - BORDER_WIDTH / 2,
                gridSettings.width * CELL_SIZE + BORDER_WIDTH,
                CELL_SIZE + BORDER_WIDTH * 2 - BORDER_WIDTH / 2
            );
            ctx.rect(
                gridSettings.width * CELL_SIZE - CELL_SIZE - BORDER_WIDTH / 2,
                0,
                CELL_SIZE + BORDER_WIDTH * 2 - BORDER_WIDTH / 2,
                gridSettings.height * CELL_SIZE + BORDER_WIDTH
            );
            ctx.rect(
                0,
                0,
                CELL_SIZE + BORDER_WIDTH * 2 - BORDER_WIDTH / 2,
                gridSettings.height * CELL_SIZE + BORDER_WIDTH
            );
            ctx.closePath();
            ctx.fill();
        }
    }

    function createGridView(
        parts: Part[],
        requirements: Requirement[],
        cells: number[],
        gridSettings: GridSettings
    ) {
        const el = document.createElement("canvas");
        el.width = gridSettings.width * CELL_SIZE + BORDER_WIDTH;
        el.height = gridSettings.height * CELL_SIZE + BORDER_WIDTH;

        const ctx = el.getContext("2d");
        drawGridView(ctx, parts, requirements, cells, gridSettings);

        return el;
    }

    let solver: Solver | null = null;

    function updateResults() {
        results.innerHTML = "";
        noResults.style.display = "none";

        if (requirements.length == 0) {
            return;
        }

        if (solver != null) {
            solver.kill();
            solver = null;
        }
        solver = new Solver(parts, requirements, gridSettings);

        (async () => {
            let found = false;

            try {
                for await (const solution of solver) {
                    found = true;
                    const cells = placeAll(
                        parts,
                        requirements,
                        solution,
                        gridSettings
                    );

                    const wrapper = document.createElement("div");
                    results.appendChild(wrapper);
                    wrapper.appendChild(
                        createGridView(parts, requirements, cells, gridSettings)
                    );
                }
            } finally {
                solver.kill();
                solver = null;
            }

            if (!found) {
                noResults.style.display = "";
            }
        })();
    }

    class Solver {
        worker: Worker;
        args: {
            parts: Part[];
            requirements: Requirement[];
            gridSettings: GridSettings;
        };

        constructor(
            parts: Part[],
            requirements: Requirement[],
            gridSettings: GridSettings
        ) {
            this.worker = new Worker(new URL("./worker.ts", import.meta.url));
            this.args = { parts, requirements, gridSettings };
        }

        async *[Symbol.asyncIterator]() {
            const ready = await new Promise<{ type: string }>((resolve) => {
                const worker = this.worker;
                worker.addEventListener("message", function eh(msg) {
                    worker.removeEventListener("message", eh);
                    resolve(msg.data);
                });
            });
            if (ready.type != "ready") {
                throw "not ready";
            }

            this.worker.postMessage({
                type: "init",
                args: this.args,
            });

            while (true) {
                const solution = await new Promise<{
                    value: Solution;
                    done: boolean;
                }>((resolve) => {
                    const worker = this.worker;
                    worker.addEventListener("message", function eh(msg) {
                        worker.removeEventListener("message", eh);
                        resolve(msg.data);
                    });
                    worker.postMessage({ type: "next" });
                });
                if (solution.done) {
                    break;
                }
                yield solution.value;
            }
        }

        kill() {
            this.worker.terminate();
        }
    }

    interface CompressedRequirements {
        i: number;
        c: number;
        b: number;
        z: number;
    }

    function compressRequirements(
        reqs: Requirement[]
    ): CompressedRequirements[] {
        return reqs.map((req) => ({
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
        }));
    }

    function uncompressRequirements(
        cs: CompressedRequirements[]
    ): Requirement[] {
        return cs.map((cr) => ({
            partIndex: cr.i,
            constraint: {
                onCommandLine: cr.c === 1 ? true : cr.c === 0 ? false : null,
                bugged: cr.b === 1 ? true : cr.b === 0 ? false : null,
                compressed: cr.z === 1 ? true : cr.z === 0 ? false : null,
            },
        }));
    }

    function loadHash() {
        const hash = decodeURIComponent(location.hash.slice(1));
        const reqs2 =
            hash != "" ? uncompressRequirements(JSON.parse(hash)) : [];
        if (isEqual(requirements, reqs2)) {
            return;
        }
        requirements = reqs2;
        update();
    }

    window.onhashchange = () => {
        loadHash();
    };

    function update() {
        location.hash = JSON.stringify(compressRequirements(requirements));

        requirementsEl.innerHTML = "";

        for (let i = 0; i < requirements.length; ++i) {
            const requirement = requirements[i];

            const part = parts[requirement.partIndex];

            const li = document.createElement("li");
            requirementsEl.appendChild(li);
            li.className = "list-group-item";

            const headerEl = document.createElement("div");
            li.appendChild(headerEl);
            headerEl.className = "mb-2";

            const deleteButton = document.createElement("button");
            deleteButton.className = "btn btn-danger btn-sm";
            deleteButton.innerHTML = `<i class="bi bi-x"></i>`;
            deleteButton.onclick = ((i: number) => {
                requirements.splice(i, 1);
                update();
            }).bind(null, i);
            headerEl.appendChild(deleteButton);

            headerEl.appendChild(document.createTextNode(" "));
            headerEl.appendChild(document.createTextNode(part.name));

            const constraintsEl = document.createElement("div");
            li.appendChild(constraintsEl);

            constraintsEl.className = "row g-2";

            {
                const wrapperEl = document.createElement("div");
                wrapperEl.className = "col";
                constraintsEl.appendChild(wrapperEl);
                wrapperEl.appendChild(
                    createConstraintDropdown(
                        "on command line・コマンドライン上",
                        requirement.constraint.onCommandLine,
                        ((i: number, v: boolean | null) => {
                            requirements[i].constraint.onCommandLine = v;
                            updateResults();
                        }).bind(this, i)
                    )
                );
            }
            {
                const wrapperEl = document.createElement("div");
                wrapperEl.className = "col";
                constraintsEl.appendChild(wrapperEl);
                wrapperEl.appendChild(
                    createConstraintDropdown(
                        "cause bug・バグを引き起こす",
                        requirement.constraint.bugged,
                        ((i: number, v: boolean | null) => {
                            requirements[i].constraint.bugged = v;
                            updateResults();
                        }).bind(this, i)
                    )
                );
            }
            {
                const wrapperEl = document.createElement("div");
                wrapperEl.className = "col";
                constraintsEl.appendChild(wrapperEl);
                wrapperEl.appendChild(
                    createConstraintDropdown(
                        "compress・圧縮",
                        requirement.constraint.compressed,
                        ((i: number, v: boolean | null) => {
                            requirements[i].constraint.compressed = v;
                            updateResults();
                        }).bind(this, i)
                    )
                );
            }
        }

        updateResults();
    }

    document.getElementById("reset")!.onclick = () => {
        requirements.splice(0, requirements.length);
        update();
    };

    loadHash();
}

main();
