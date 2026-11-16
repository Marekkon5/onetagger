// Exchange the launch capability for an HttpOnly session cookie before mounting the application.
export async function establishSession(): Promise<void> {
    const match = window.location.hash.match(/^#session=([a-f0-9]{64})$/);
    if (match) {
        // Fragments are not sent in HTTP requests. Remove it before the router and UI start.
        history.replaceState(null, '', `${location.pathname}${location.search}#/`);
        const response = await fetch('/session', {
            method: 'POST',
            credentials: 'same-origin',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ token: match[1] }),
        });
        if (!response.ok) throw new Error('This OneTagger session link is invalid or expired.');
    } else {
        const response = await fetch('/session', { credentials: 'same-origin' });
        if (!response.ok) throw new Error('Open OneTagger using the session link printed by the running application.');
    }
}
