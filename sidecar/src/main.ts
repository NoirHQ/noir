import { App } from "./app";
import config from "config";

async function main() {
	const app = new App(config);
	await app.initialize();
	await app.start();
}

main().catch((e: unknown) => console.error(e));
