import { listen, TauriEvent } from 'https://esm.sh/v131/@tauri-apps/api@1.4.0/event'

async function doCapture(id) {
    await window.__TAURI__.invoke("begin_capture", { "windowId": id });
    document.getElementById("tile-container-outer").hidden = true;
    document.getElementById("chat-panel-outer").hidden = false;
    document.getElementById("snippy-text").innerText = "Ask me anything you want and I'll try to help!";
}

function make_tile(image_base64, id, title) {
    let content_image = document.createElement("img");
    content_image.src = "data:image/png;base64, " + image_base64;
    content_image.className = "tile-image";

    let content_title = document.createElement("p");
    content_title.innerText = title;
    content_title.className = "tile-title";
    
    let content_div = document.createElement("div");
    content_div.className = "tile-contents";
    content_div.appendChild(content_image);
    content_div.appendChild(content_title)

    let box_div = document.createElement("div");
    box_div.className = "tile";
    box_div.appendChild(content_div);
    box_div.id = "box" + id;

    box_div.addEventListener("click", function () {
        console.log("capture window + " + id + " clicked! capturing...");
        doCapture(id);
    });

    document.getElementById("tile-container").appendChild(box_div);
}

listen("window_found", async event => {
    //log("window_found", event);
    let window = event.payload;
    make_tile(window.thumbnail, window.id, window.title);
});

function gptMarkdownToHtml(text) {
    const codeBlockRegex = /```(\w+)?\n([\s\S]*?)```/g;

    return text.replace(codeBlockRegex, (match, lang, code) => {
        // Encode <, >, and & for proper display in HTML
        const encodedCode = code.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
        return `<pre><code class="${lang || ''}">${encodedCode}</code></pre>`;
    });
}

function addMessage(text, isSent) {
    const chatHistory = document.getElementById("chat-history");
    var newMessage = document.createElement('div');
    newMessage.classList.add('chat-message', isSent ? 'sent' : 'received');
    if (!isSent) {
        try {
            var html = marked.marked(text);
            newMessage.innerHTML = html;
        } catch {
            newMessage.innerHTML = gptMarkdownToHtml(text);
        }
    } else {
        newMessage.textContent = text;
    }
    chatHistory.appendChild(newMessage);
    // Scroll to the bottom of the chat history
    chatHistory.scrollTop = chatHistory.scrollHeight;
}

async function sendMessage() {
    const sendText = document.getElementById("sendText");
    const text = sendText.value.trim();
    sendText.value = "";
    if (text !== "") {
        addMessage(text, true);
        let response = await window.__TAURI__.invoke("send_message", {msg: text});
        addMessage(response, false);
    }
}

document.addEventListener("DOMContentLoaded", async (e) => {

    document.getElementById("sendText").addEventListener("keypress", (event) => {
        if (event.key === "Enter") {
            sendMessage();
        }
    });
    document.getElementById("sendButton").addEventListener("click", (event) => {
        sendMessage();
    });

    let windows_json_string = await window.__TAURI__.invoke("get_windows", {req: 1});
    //let windows = JSON.parse(windows_json_string);
    //windows.forEach(function (window) {
    //    make_tile(window.thumbnail, window.id);
    //});
});