import data from "./bn6.json";
import { convertParts, GridSettings, Part, placeAll, Requirement, solve } from "./solver";

const parts = convertParts(data.parts, 7, 7);

const partSelect = document.getElementById("part-select")! as HTMLSelectElement;

const results = document.getElementById("results")!;

for (let i = 0; i < parts.length; ++i) {
    const part = parts[i];

    const option = document.createElement("option");
    partSelect.appendChild(option);
    option.value = i.toString();
    option.textContent = `${part.name}・${part.nameJa}`;
}

const requirements: Requirement[] = [];

const requirementsEl = document.getElementById("requirements")!;

partSelect.onchange = () => {
    const partIndex = parseInt(partSelect.value, 10);
    const part = parts[partIndex];

    requirements.push({
        partIndex,
        constraint: {
            bugged: null,
            compressed: null,
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

    // First pass: draw background.
    ctx.fillStyle = BG_FILL_COLOR;
    ctx.fillRect(
        0,
        0,
        gridSettings.width * CELL_SIZE,
        gridSettings.height * CELL_SIZE
    );

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

            const px = x * CELL_SIZE;
            const py = y * CELL_SIZE;

            ctx.fillStyle = color.solid;
            ctx.strokeStyle = color.plus;

            ctx.fillRect(px, py, CELL_SIZE, CELL_SIZE);

            if (!part.isSolid) {
                ctx.strokeRect(px, py, CELL_SIZE, 1);
                ctx.strokeRect(px, py + CELL_SIZE / 2, CELL_SIZE, 1);
                ctx.strokeRect(px, py + CELL_SIZE, CELL_SIZE, 1);
                ctx.strokeRect(px, py, 1, CELL_SIZE);

                ctx.strokeRect(px + CELL_SIZE / 2, py, 1, CELL_SIZE);
                ctx.strokeRect(px + CELL_SIZE, py, 1, CELL_SIZE);
            }
        }
    }

    // Third pass: draw borders.
    for (let y = 0; y < gridSettings.height; ++y) {
        for (let x = 0; x < gridSettings.width; ++x) {
            const cell = cells[y * gridSettings.width + x];
        }
    }

    // Fourth pass: draw command line.

    // Fifth pass: draw out of bounds overlay.
}

function createGridView(
    parts: Part[],
    requirements: Requirement[],
    cells: number[],
    gridSettings: GridSettings
) {
    const el = document.createElement("canvas");
    el.width = gridSettings.width * CELL_SIZE;
    el.height = gridSettings.height * CELL_SIZE;

    const ctx = el.getContext("2d");
    drawGridView(ctx, parts, requirements, cells, gridSettings);

    return el;
}

function updateResults() {
    results.innerHTML = "";

    if (requirements.length == 0) {
        return;
    }

    let i = 0;
    const solver = solve(parts, requirements, gridSettings);
    for (const solution of solver) {
        const cells = placeAll(parts, requirements, solution, gridSettings);

        const wrapper = document.createElement("div");
        results.insertBefore(wrapper, results.firstChild);

        wrapper.appendChild(
            createGridView(parts, requirements, cells, gridSettings)
        );

        ++i;
        if (i >= 5) {
            // TODO
            break;
        }
    }
}

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

const resetButton = document.getElementById("reset")!;

resetButton.onclick = () => {
    requirements.splice(0, requirements.length);
    update();
};
