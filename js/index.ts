import data from "./bn6.json";
import { convertParts, GridSettings, placeAll, Requirement, solve } from "./solver";

const parts = convertParts(data.parts, 7, 7);

const partSelect = document.getElementById("part-select")! as HTMLSelectElement;

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
        [-1, "🤷 optional・任意"],
        [0, "❌ must not・不要"],
        [1, "✅ must・必要"],
    ] as [number, string][]) {
        const option = document.createElement("option");
        select.appendChild(option);
        option.value = v.toString();
        option.textContent = text;
    }

    select.className = "form-select";
    select.onchange = () => {
        const v = parseInt(select.value, 10);
        onchange(v === 1 ? true : v === 0 ? false : null);
    };
    select.value = (
        initialValue === true ? 1 : initialValue === false ? 0 : -1
    ).toString();

    const label = document.createElement("label");
    el.append(label);
    label.textContent = title;

    return el;
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
                    () => {}
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
                    () => {}
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
                    () => {}
                )
            );
        }
    }
}

const resetButton = document.getElementById("reset")!;

resetButton.onclick = () => {
    requirements.splice(0, requirements.length);
    update();
};
