// Serialized by chrome.scripting: this function intentionally has no dependencies.
// It inspects only the composer, never conversation history, and never submits.
export async function insertIntoComposer(options) {
  const providers = {
    chatgpt:{host:"chatgpt.com", selectors:['#prompt-textarea[contenteditable="true"]','textarea#prompt-textarea','[data-testid="composer"] [contenteditable="true"]','main form textarea']},
    deepseek:{host:"chat.deepseek.com", selectors:['textarea#chat-input','textarea[placeholder*="DeepSeek"]','main textarea','textarea']},
    claude:{host:"claude.ai", selectors:['.tiptap[contenteditable="true"]','[contenteditable="true"][role="textbox"]','.ProseMirror[contenteditable="true"]']},
    gemini:{host:"gemini.google.com", selectors:['rich-textarea [contenteditable="true"]','.ql-editor[contenteditable="true"]']}
  };
  const config = providers[options.provider];
  if (!config || location.protocol !== "https:" || location.hostname !== config.host) return {ok:false,code:"wrong-site"};
  if (typeof options.prompt !== "string" || !options.prompt.trim() || options.prompt.length > 200000) return {ok:false,code:"invalid-prompt"};
  if (!["empty","append","replace"].includes(options.mode ?? "empty")) return {ok:false,code:"invalid-mode"};
  const visible = el => !!el.getClientRects().length && getComputedStyle(el).visibility !== "hidden" && !el.closest('[aria-hidden="true"], [inert]');
  if ([...document.querySelectorAll('[role="dialog"][aria-modal="true"]')].some(visible)) return {ok:false,code:"dialog-open"};
  let candidates = [];
  for (const selector of config.selectors) {
    candidates = [...document.querySelectorAll(selector)].filter(visible);
    if (candidates.length) break;
  }
  let editor = candidates.length === 1 ? candidates[0] : candidates.find(el => el === document.activeElement || el.contains(document.activeElement));
  if (!editor) return {ok:false,code:candidates.length ? "ambiguous-editor" : "no-editor"};
  if (editor.disabled || editor.readOnly || editor.getAttribute("aria-disabled") === "true") return {ok:false,code:"disabled-editor"};
  const textarea = editor instanceof HTMLTextAreaElement;
  const read = () => textarea ? editor.value : editor.innerText ?? editor.textContent ?? "";
  // Rich-text editors can represent a newline as adjacent paragraphs; innerText
  // then adds blank lines. Compare their content without that layout spacing.
  const clean = value => {
    const text=value.replace(/\r\n/g,"\n").replace(/[\u200b\ufeff]/g,"").trim();
    return textarea ? text : text.replace(/\n{2,}/g,"\n");
  };
  const before = read();
  if (clean(before) === clean(options.prompt) || ((options.mode ?? "empty") === "append" && clean(before).endsWith(clean(options.prompt)))) return {ok:true,code:"already-inserted"};
  const token = async value => [...new Uint8Array(await crypto.subtle.digest("SHA-256",new TextEncoder().encode(value)))].map(n=>n.toString(16).padStart(2,"0")).join("");
  if (before.trim()) {
    const draftToken = await token(before);
    if ((options.mode ?? "empty") === "empty" || options.draftToken !== draftToken) return {ok:false,code:"draft-not-empty",draftToken};
    if (read() !== before) return {ok:false,code:"draft-changed"};
  }
  const replacement = options.mode === "append" && before.trim() ? `${before}\n\n${options.prompt}` : options.prompt;
  const event = new InputEvent("beforeinput",{bubbles:true,cancelable:true,inputType:"insertText",data:replacement});
  if (!editor.dispatchEvent(event)) return {ok:false,code:"site-rejected"};
  if (read() !== before) return {ok:false,code:"draft-changed"};
  editor.focus();
  if (textarea) {
    Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype,"value").set.call(editor,replacement);
    editor.dispatchEvent(new InputEvent("input",{bubbles:true,inputType:"insertText",data:replacement}));
    editor.dispatchEvent(new Event("change",{bubbles:true}));
    editor.setSelectionRange(replacement.length,replacement.length);
  } else {
    const selection = window.getSelection(); const range = document.createRange();
    range.selectNodeContents(editor); selection.removeAllRanges(); selection.addRange(range);
    // Native editing preserves the state of ProseMirror/Quill-style editors.
    if (!document.execCommand("insertText",false,replacement)) return {ok:false,code:"site-rejected"};
  }
  await new Promise(resolve => setTimeout(resolve,120));
  if (!editor.isConnected || clean(read()) !== clean(replacement)) return {ok:false,code:"site-rejected"};
  return {ok:true,code:"inserted"};
}
