import hljs from "highlight.js/lib/core";
import typescript from "highlight.js/lib/languages/typescript";
import javascript from "highlight.js/lib/languages/javascript";
import python from "highlight.js/lib/languages/python";
import rust from "highlight.js/lib/languages/rust";
import json from "highlight.js/lib/languages/json";
import bash from "highlight.js/lib/languages/bash";
import powershell from "highlight.js/lib/languages/powershell";
import css from "highlight.js/lib/languages/css";
import xml from "highlight.js/lib/languages/xml";
import sql from "highlight.js/lib/languages/sql";
import yaml from "highlight.js/lib/languages/yaml";
import { h, clear } from "./dom";

for (const [name, grammar] of Object.entries({typescript,javascript,python,rust,json,bash,powershell,css,xml,sql,yaml})) hljs.registerLanguage(name, grammar);
const extensions: Record<string,string> = {ts:"typescript",tsx:"typescript",js:"javascript",jsx:"javascript",mjs:"javascript",py:"python",rs:"rust",json:"json",sh:"bash",ps1:"powershell",css:"css",html:"xml",htm:"xml",xml:"xml",svg:"xml",sql:"sql",yaml:"yaml",yml:"yaml"};
type Entry = {label:string;detail:string;input?:Record<string,unknown>};

function code(text:string, language:string): HTMLElement {
  const el = h("code");
  // Only the highlighter's escaped output is inserted as HTML; source and paths
  // never become markup. Unknown formats remain literal text.
  if (hljs.getLanguage(language)) el.innerHTML = hljs.highlight(text,{language,ignoreIllegals:true}).value;
  else el.textContent = text;
  return el;
}

export function renderActivity(host:HTMLElement, entry?:Entry): void {
  clear(host);
  if (!entry?.input || !Object.keys(entry.input).length) { host.append(h("pre",{class:"activity-plain",text:entry?.detail??"Tool inputs and replies appear here as this session works."})); return; }
  const input=entry.input;
  const path=String(input.file_path??input.path??input.filename??"");
  const language=extensions[path.split(".").at(-1)?.toLowerCase()??""]??"";
  if(path) host.append(h("div",{class:"code-file",text:path}));
  let remaining=12000;
  for(const [field,value] of Object.entries(input)) {
    if(["file_path","path","filename"].includes(field)) continue;
    if(remaining<=0) {host.append(h("div",{class:"code-field",text:"Preview truncated"}));break;}
    const raw=typeof value==="string"?value:JSON.stringify(value,null,2)??String(value);
    const text=raw.slice(0,remaining);remaining-=text.length;
    const patch=/^(\*\*\* Begin Patch|diff --git|--- |@@ )/m.test(text);
    host.append(h("div",{class:"code-field",text:patch?"Changes":field==="old_string"?"Removed":field==="new_string"?"Added":field.replaceAll("_"," ")}));
    const block=h("pre",{class:"code-block"});
    if(patch) {
      let currentLanguage=language;
      for(const line of text.split("\n")) {
        const file=line.match(/^(?:\*\*\* (?:Update|Add|Delete) File: |\+\+\+ b\/)(.+)$/);
        if(file)currentLanguage=extensions[file[1].split(".").at(-1)?.toLowerCase()??""]??"";
        const meta=/^(\*\*\*|@@|diff --git|index |--- |\+\+\+ )/.test(line);
        const kind=meta?"meta":line.startsWith("+")?"added":line.startsWith("-")?"removed":"context";
        const prefix=meta?"":kind==="context"?" ":line[0];
        block.append(h("div",{class:`code-line ${kind}`},h("span",{class:"code-gutter",text:prefix}),code(meta?line:line.slice(/^[ +\-]/.test(line)?1:0),meta?"":currentLanguage)));
      }
    } else {
      const diff=field==="old_string"?"removed":field==="new_string"?"added":"";
      const lang=language||(["command","cmd"].includes(field)?(/powershell|Get-|Set-|\$env:/.test(text)?"powershell":"bash"):field==="code"?"javascript":typeof value==="object"?"json":"");
      block.classList.add(...(diff?[diff]:[]));
      block.append(code(text,lang));
    }
    host.append(block);
    if(text.length<raw.length)host.append(h("div",{class:"code-field",text:"Preview truncated"}));
  }
}
