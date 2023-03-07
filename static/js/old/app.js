//import { createApp } from "https://unpkg.com/vue@3/dist/vue.esm-browser.js";
var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
const ProjPart = {
    name: "proj-part",
    data() {
        return {};
    },
    props: {
        thisPart: Object,
        thisPath: {
            type: String,
            default: "",
        },
    },
    methods: {
        newPart() {
            return __awaiter(this, void 0, void 0, function* () { });
        },
    },
    // TODO: width
    template: `
<li>
  <details v-if="thisPart.subparts.length">
    <summary>{{thisPart.name}}<button @click="app.newPartPopup(thisPath)">+</button></summary>
    <ul>
        <proj-part
          v-for="(subpart, index) in thisPart.subparts"
          :key="index"
          :part="subpart"
          :path="thisPath+'\n'+subpart.path"
        >
        </proj-part>
    </ul>
  </details>
  <span v-else>{{thisPart.name}}<button>+</button></span>
</li>
  `,
};
export const App = {
    data() {
        return {
            loggingIn: true,
            errMsg: "",
            email: "johnietre@gmail.com",
            password: "Rj385637",
            sessionId: "",
            projects: [],
            part: newClientPart(),
            partPath: "",
        };
    },
    components: {
        ProjPart: (() => {
            ProjPart.data = () => (Object.assign({ app: this }, ProjPart.data()));
            return ProjPart;
        })(),
    },
    beforeMount() {
        return __awaiter(this, void 0, void 0, function* () {
            // TODO: Figure out better way to handle sessionId storage (check response code?)
            if (yield this.getProjects()) {
                this.sessionId = "1";
            }
            this.errMsg = "";
        });
    },
    methods: {
        sendCreds() {
            return __awaiter(this, void 0, void 0, function* () {
                this.errMsg = "";
                const path = (this.loggingIn) ? "/login" : "register";
                const url = new URL(path, window.location.href);
                const resp = yield fetch(url.toString(), {
                    method: "POST",
                    headers: {
                        "PROJ-TRAK-EMAIL": this.email,
                        "PROJ-TRAK-PWD": this.password,
                    }
                });
                this.password = "";
                const text = yield resp.text();
                if (!resp.ok) {
                    this.errMsg = text || resp.statusText;
                    return;
                }
                this.sessionId = text;
                if (this.sessionId == "") {
                    console.log("OK response but no session id", resp);
                    return;
                }
                // TODO: Figure out better way to handle sessionId storage (check response code?)
                if (!(yield this.getProjects()))
                    this.sessionId = "";
            });
        },
        // Returns true if the operation was ok
        getProjects() {
            return __awaiter(this, void 0, void 0, function* () {
                const url = new URL("/projects", window.location.href);
                const resp = yield fetch(url.toString(), {
                    method: "GET",
                    headers: { "PROJ-TRAK-SESS": this.sessionId },
                    credentials: "same-origin",
                });
                if (!resp.ok) {
                    const text = (yield resp.text()) || resp.statusText;
                    console.log("error getting projects:", text);
                    return false;
                }
                this.projects = fromServerParts(yield resp.json());
                return true;
            });
        },
        newPart() {
            return __awaiter(this, void 0, void 0, function* () {
                this.part.name = this.part.name.trim();
                if (this.part.name == "") {
                    // TODO: Display error
                    return;
                }
                const url = new URL("/projects", window.location.href);
                const resp = yield fetch(url.toString(), {
                    method: "POST",
                    // TODO: Convert part to server part
                    body: this.pathPath + "\n" + JSON.stringify(this.part),
                });
                if (!resp.ok) {
                    console.log("error adding part:", (yield resp.text()) || resp.statusText);
                    return;
                }
                // TODO: Add part to parts
            });
        },
        newPartPopup(partPath = "") {
            this.partPath = partPath;
            const popup = document.querySelector("#part-popup");
            popup.style.display = "block";
        },
        cancelNewPart() {
            const popup = document.querySelector("#part-popup");
            popup.style.display = "none";
            this.part = newClientPart();
            this.partPath = "";
        },
        login() {
            return __awaiter(this, void 0, void 0, function* () { });
        },
        register() {
            return __awaiter(this, void 0, void 0, function* () { });
        },
    },
};
;
;
function newClientPart(name, description, deadline, completedAt) {
    return { name: name !== null && name !== void 0 ? name : "", description: description, deadline: deadline, completedAt: completedAt, subparts: [] };
}
;
function fromServerParts(parts) {
    let res = [];
    for (var name in parts) {
        res.push({
            name: name,
            subparts: fromServerParts(parts[name].subparts),
        });
    }
    res.sort((a, b) => (a.name.toLowerCase() > b.name.toLowerCase()) ? 1 : -1);
    return res;
}
function toServerParts(parts) {
    let res = {};
    for (var part of parts) {
        /*
        res[part.name] = {
          name: part.name,
          subparts: toServerParts(part.subparts),
        };
       */
        res[part.name] = toServerPart(part);
    }
    return res;
}
function toServerPart(part) {
    return Object.assign(Object.assign({}, part), { subparts: toServerParts(part.subparts) });
}
