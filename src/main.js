import { mount } from "svelte";
// Bundle JetBrains Mono locally (woff2) so the terminal/editor look identical
// on every machine regardless of installed fonts.
import "@fontsource/jetbrains-mono/400.css";
import "@fontsource/jetbrains-mono/700.css";
import "@fontsource/jetbrains-mono/400-italic.css";
import "@fontsource/jetbrains-mono/700-italic.css";
import "./app.css";
import App from "./App.svelte";

const app = mount(App, { target: document.getElementById("app") });

export default app;
