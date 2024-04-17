import { listen, TauriEvent } from 'https://esm.sh/v131/@tauri-apps/api@1.4.0/event'

function make_tile(image_base64, id) {
    let content_image = document.createElement("img");
    content_image.src = "data:image/png;base64, " + image_base64;
    content_image.className = "tile-image";
    
    let content_div = document.createElement("div");
    content_div.className = "tile-contents";
    content_div.appendChild(content_image);

    let box_div = document.createElement("div");
    box_div.className = "tile";
    box_div.appendChild(content_div);
    box_div.id = "box" + id;

    box_div.addEventListener("click", async function () {
        console.log("capture window + " + id + " clicked! capturing...");
        await window.__TAURI__.invoke("begin_capture", { "windowId": id });
    });

    document.getElementById("tile-container").appendChild(box_div);
}

listen("window_found", async event => {
    //log("window_found", event);
    let window = event.payload;
    make_tile(window.thumbnail, window.id);
});

document.addEventListener("DOMContentLoaded", async (e) => {
    let windows_json_string = await window.__TAURI__.invoke("get_windows", {req: 1});
    //let windows = JSON.parse(windows_json_string);
    //windows.forEach(function (window) {
    //    make_tile(window.thumbnail, window.id);
    //});
});