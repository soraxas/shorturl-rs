const refreshData = async () => {
    let data = await fetch("/api/all").then(res => res.text());
    data = data
        .split("\n")
        .filter(line => line !== "")
        .map(line => line.split(","))
        .map(arr => ({
            long: arr[1],
            short: arr[0]
        }));

    displayData(data);
};

const displayData = (data) => {
    const table = document.querySelector("#url-table");
    table.innerHTML = ''; // Clear
    data.map(TR)
        .forEach(tr => table.appendChild(tr));
};

const TR = (row) => {
    const tr = document.createElement("tr");
    const longTD = TD(A(row.long));
    const shortTD = TD(A_INT(row.short));
    const btn = deleteButton(row.short);

    tr.appendChild(longTD);
    tr.appendChild(shortTD);
    tr.appendChild(btn);

    return tr;
};

const A = (s) => `<a href='${s}'>${s}</a>`;
const A_INT = (s) => `<a href='/${s}'>${window.location.host}/${s}</a>`;

const deleteButton = (shortUrl) => {
    const btn = document.createElement("button");

    btn.innerHTML = "&times;";

    btn.onclick = e => {
        e.preventDefault();
        fetch(`/api/${shortUrl}`, {
            method: "DELETE"
        }).then(_ => refreshData());
    };

    return btn;
};

const TD = (s) => {
    const td = document.createElement("td");
    td.innerHTML = s;
    return td;
};

const submitForm = () => {
    const form = document.forms.namedItem("new-url-form");
    const longUrl = form.elements["longUrl"];
    const shortUrl = form.elements["shortUrl"];

    const url = `/api/new`;

    fetch(url, {
        method: "POST",
        body: `${longUrl.value};${shortUrl.value}`
    })
    .then(_ => {
        longUrl.value = "";
        shortUrl.value = "";

        refreshData();
    });

};

(async () => {
    await refreshData();
    const form = document.forms.namedItem("new-url-form");
    form.onsubmit = e => {
        e.preventDefault();
        submitForm();
    }
})();
