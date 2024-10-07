export interface ApiService {
}

export class ApiServices {
	services: Map<string, ApiService>;

	constructor() {
		this.services = new Map<string, ApiService>();
	}

	public get<T extends ApiService>(name: string): T {
		return this.services.get(name) as T;
	}

	public set(name: string, service: ApiService) {
		if (this.get(name)) {
			throw new Error(`Already reserved service name: ${name}`);
		}
		this.services.set(name, service);
	}
}
