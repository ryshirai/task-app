import autoAdapter from '@sveltejs/adapter-auto';

let adapter = autoAdapter;
try {
	const module = await import('@sveltejs/adapter-cloudflare');
	adapter = module.default;
} catch (error) {
	// Allow local builds in constrained environments where Cloudflare adapter is not installed yet.
	if (error?.code !== 'ERR_MODULE_NOT_FOUND') {
		throw error;
	}
}

/** @type {import('@sveltejs/kit').Config} */
const config = {
	kit: {
		adapter: adapter()
	}
};

export default config;
