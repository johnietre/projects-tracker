//import { createApp } from "https://unpkg.com/vue@3/dist/vue.esm-browser.js";

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
    async newPart() {},
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
      ProjPart.data = () => ({app: this, ...ProjPart.data()});
      return ProjPart;
    })(),
  },

  async beforeMount() {
    // TODO: Figure out better way to handle sessionId storage (check response code?)
    if (await this.getProjects()) {
      this.sessionId = "1";
    }
    this.errMsg = "";
  },

  methods: {
    async sendCreds() {
      this.errMsg = "";
      const path = (this.loggingIn) ? "/login" : "register";
      const url = new URL(path, window.location.href);
      const resp = await fetch(url.toString(), {
        method: "POST",
        headers: {
          "PROJ-TRAK-EMAIL": this.email,
          "PROJ-TRAK-PWD": this.password,
        }
      });
      this.password = "";
      const text = await resp.text();
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
      if (!(await this.getProjects()))
        this.sessionId = "";
    },

    // Returns true if the operation was ok
    async getProjects(): Promise<boolean> {
      const url = new URL("/projects", window.location.href);
      const resp = await fetch(url.toString(), {
        method: "GET",
        headers: {"PROJ-TRAK-SESS": this.sessionId},
        credentials: "same-origin",
      });
      if (!resp.ok) {
        const text = (await resp.text()) || resp.statusText;
        console.log("error getting projects:", text);
        return false;
      }
      this.projects = fromServerParts(await resp.json());
      return true;
    },

    async newPart() {
      this.part.name = this.part.name.trim();
      if (this.part.name == "") {
        // TODO: Display error
        return;
      }
      const url = new URL("/projects", window.location.href);
      const resp = await fetch(url.toString(), {
        method: "POST",
        // TODO: Convert part to server part
        body: this.pathPath + "\n" + JSON.stringify(this.part),
      });
      if (!resp.ok) {
        console.log("error adding part:", (await resp.text()) || resp.statusText);
        return;
      }
      // TODO: Add part to parts
    },

    newPartPopup(partPath: string = "") {
      this.partPath = partPath;
      const popup = document.querySelector("#part-popup") as HTMLElement;
      popup.style.display = "block";
    },

    cancelNewPart() {
      const popup = document.querySelector("#part-popup") as HTMLElement;
      popup.style.display = "none";
      this.part = newClientPart();
      this.partPath = "";
    },

    async login() {},

    async register() {},
  },
};

interface Part {
  name: string;
  description?: string;
  deadline?: number;
  completedAt?: number;
};

interface ClientPart extends Part {
  subparts: ClientParts;
};

function newClientPart(name?: string, description?: string, deadline?: number, completedAt?: number): ClientPart {
  return {name: name ?? "", description: description, deadline: deadline, completedAt: completedAt, subparts: []};
}

interface ServerPart extends Part {
  subparts: ServerParts;
};

type ClientParts = ClientPart[];

type ServerParts = {
  [key: string]: ServerPart;
};

function fromServerParts(parts: ServerParts): ClientParts {
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

function toServerParts(parts: ClientParts): ServerParts {
  let res = {};
  for (var part of parts) {
    res[part.name] = toServerPart(part);
  }
  return res;
}

function toServerPart(part: ClientPart): ServerPart {
  return {
    ...part,
    subparts: toServerParts(part.subparts),
  };
}
