import isEqual from "lodash-es/isEqual";

import {
    convertParts,
    GridSettings,
    Part,
    placeAll,
    Requirement,
    Solution,
} from "./solver";

async function main() {
    let requirements: Requirement[] = [];

    const queryParams = new URLSearchParams(location.search);
    const game = queryParams.get("game") || "bn6";

    document
        .getElementById("games-nav")
        .querySelector(`a[href='?game=${game}']`)
        .classList.add("active");

    const data = await import(`./${game}.json`);
    const gridSettings: GridSettings = data.gridSettings;

    const parts = convertParts(
        data.parts,
        gridSettings.height,
        gridSettings.width
    );

    const partSelect = document.getElementById(
        "part-select"
    )! as HTMLSelectElement;
    partSelect.disabled = false;

    const results = document.getElementById("results")!;
    const noResults = document.getElementById("no-results");
    const noRequirements = document.getElementById("no-requirements");

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
                compressed: !isEqual(part.compressedMask, part.uncompressedMask)
                    ? true
                    : false,
                onCommandLine: part.isSolid ? true : null,
            },
        });
        partSelect.value = "";
        update();
    };

    function createConstraintDropdown(
        title: string,
        initialValue: boolean | null,
        disabled: boolean,
        onchange: (v: boolean | null) => void
    ) {
        const el = document.createElement("div");
        el.className = "form-floating";

        const select = document.createElement("select");
        select.disabled = disabled;
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
                CELL_SIZE + BORDER_WIDTH * 2 - BORDER_WIDTH / 2
            );
            ctx.rect(
                CELL_SIZE,
                gridSettings.height * CELL_SIZE - CELL_SIZE,
                (gridSettings.width - 2) * CELL_SIZE + BORDER_WIDTH,
                CELL_SIZE + BORDER_WIDTH * 2 - BORDER_WIDTH / 2
            );
            ctx.rect(
                gridSettings.width * CELL_SIZE - CELL_SIZE,
                CELL_SIZE,
                CELL_SIZE + BORDER_WIDTH * 2 - BORDER_WIDTH / 2,
                (gridSettings.height - 2) * CELL_SIZE + BORDER_WIDTH
            );
            ctx.rect(
                0,
                CELL_SIZE,
                CELL_SIZE + BORDER_WIDTH * 2 - BORDER_WIDTH / 2,
                (gridSettings.height - 2) * CELL_SIZE + BORDER_WIDTH
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

    function createSpinner(gridSettings: GridSettings) {
        const wrapper = document.createElement("div");
        wrapper.className = "d-flex justify-content-center align-items-center";
        wrapper.style.width = `${
            gridSettings.width * CELL_SIZE + BORDER_WIDTH
        }px`;
        wrapper.style.height = `${
            gridSettings.height * CELL_SIZE + BORDER_WIDTH
        }px`;

        const spinner = document.createElement("div");
        wrapper.appendChild(spinner);

        spinner.className = "spinner-border";
        return wrapper;
    }

    let solver: Solver | null = null;

    function updateResults() {
        location.hash = JSON.stringify(compressRequirements(requirements));

        results.innerHTML = "";
        noResults.style.display = "none";
        noRequirements.style.display = "none";

        if (requirements.length == 0) {
            noRequirements.style.display = "";
            return;
        }

        if (solver != null) {
            solver.kill();
            solver = null;
        }
        solver = new Solver(parts, requirements, gridSettings);

        const spinner = createSpinner(gridSettings);
        results.appendChild(spinner);

        const it = solver[Symbol.asyncIterator]();

        (async () => {
            const { value: solution, done } = await it.next();
            if (done) {
                spinner.parentNode.removeChild(spinner);
                noResults.style.display = "";
                return;
            }

            const cells = placeAll(
                parts,
                requirements,
                solution as Solution,
                gridSettings
            );

            const wrapper = document.createElement("div");
            results.insertBefore(wrapper, spinner);
            wrapper.appendChild(
                createGridView(parts, requirements, cells, gridSettings)
            );

            const observer = new IntersectionObserver(([entry]) => {
                if (entry.intersectionRatio <= 0) {
                    return;
                }

                (async () => {
                    while (true) {
                        const { value: solution, done } = await it.next();

                        if (done) {
                            spinner.parentNode.removeChild(spinner);
                            return;
                        }

                        const cells = placeAll(
                            parts,
                            requirements,
                            solution as Solution,
                            gridSettings
                        );

                        const wrapper = document.createElement("div");
                        results.insertBefore(wrapper, spinner);
                        wrapper.appendChild(
                            createGridView(
                                parts,
                                requirements,
                                cells,
                                gridSettings
                            )
                        );

                        const clientRect = spinner.getBoundingClientRect();
                        const overscroll = 100;
                        if (clientRect.top - overscroll > window.innerHeight) {
                            break;
                        }
                    }
                })();
            });
            observer.observe(spinner);
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
            headerEl.appendChild(
                document.createTextNode(
                    `${i + 1}. ${part.name}・${part.nameJa}`
                )
            );

            const constraintsEl = document.createElement("div");
            li.appendChild(constraintsEl);

            constraintsEl.className = "row g-2";

            {
                const wrapperEl = document.createElement("div");
                wrapperEl.className = "col-xl";
                constraintsEl.appendChild(wrapperEl);
                wrapperEl.appendChild(
                    createConstraintDropdown(
                        "on command line・コマンドライン上",
                        requirement.constraint.onCommandLine,
                        false,
                        ((i: number, v: boolean | null) => {
                            requirements[i].constraint.onCommandLine = v;
                            updateResults();
                        }).bind(this, i)
                    )
                );
            }
            {
                const wrapperEl = document.createElement("div");
                wrapperEl.className = "col-xl";
                constraintsEl.appendChild(wrapperEl);
                wrapperEl.appendChild(
                    createConstraintDropdown(
                        "cause bug・バグを引き起こす",
                        requirement.constraint.bugged,
                        false,
                        ((i: number, v: boolean | null) => {
                            requirements[i].constraint.bugged = v;
                            updateResults();
                        }).bind(this, i)
                    )
                );
            }
            {
                const wrapperEl = document.createElement("div");
                wrapperEl.className = "col-xl";
                constraintsEl.appendChild(wrapperEl);
                wrapperEl.appendChild(
                    createConstraintDropdown(
                        "compress・圧縮",
                        requirement.constraint.compressed,
                        isEqual(part.compressedMask, part.uncompressedMask),
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
