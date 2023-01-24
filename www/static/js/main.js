const refreshData = async () => {
    let data = await fetch("/v1/urls", {
        headers: {
            "x-api-key": apikey,
        },
    }).then((res) => res.json());
    data = data.map((arr) => ({
        long: arr.url,
        short: arr.short_code,
    }));

    displayData(data);
};

const displayData = (data) => {
    const table = document.querySelector("#url-table");
    table.innerHTML = ""; // Clear
    data.map(TR).forEach((tr) => table.appendChild(tr));
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

    btn.onclick = (e) => {
        e.preventDefault();
        fetch(`/v1/url/${shortUrl}`, {
            method: "DELETE",
            headers: {
                "x-api-key": apikey,
            },
        }).then((_) => refreshData());
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
    const longUrl = form.elements["longUrl"].value;
    const shortUrl = form.elements["shortUrl"].value;

    const url = `/v1/url/${shortUrl}`;

    fetch(url, {
        method: "POST",
        headers: {
            "x-api-key": apikey,
            "Content-Type": "application/json",
        },

        body: JSON.stringify({
            url: longUrl,
        }),
    }).then((_) => {
        longUrl.value = "";
        shortUrl.value = "";

        refreshData();
    });
};

const params = new Proxy(new URLSearchParams(window.location.search), {
    get: (searchParams, prop) => searchParams.get(prop),
});
const apikey = params.apikey;

(async () => {
    if (apikey) {
        const test_auth = await fetch("/v1", {
            headers: {
                "x-api-key": apikey,
            },
        }).then((res) => res.text());

        if (test_auth != "ok") {
            alert("failed to authenticate!");
            return;
        }

        await refreshData();
        const form = document.forms.namedItem("new-url-form");
        form.onsubmit = (e) => {
            e.preventDefault();
            submitForm();
        };
    }
})();
