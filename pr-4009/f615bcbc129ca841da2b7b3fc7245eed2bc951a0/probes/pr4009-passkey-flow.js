async ({ tab, page }, passphrase) => {
  const click = async (name) => {
    await page.waitForFunction(target => [...document.querySelectorAll('button,[role="menuitem"]')].some(node => !node.disabled && [node.textContent, node.getAttribute('aria-label')].some(text => text?.trim().toLowerCase() === target.toLowerCase())), { timeout: 15000 }, name);
    const state = await tab.observe({ compact: true, viewportOnly: true });
    const node = state.elements.find(e => (e.role === 'button' || e.role === 'menuitem') && e.name.toLowerCase() === name.toLowerCase());
    if (!node) throw new Error(`Missing actual control: ${name}`);
    await tab.click(`role/${node.role}[name=${JSON.stringify(node.name)}]`);
  };
  const fill = async (name, value) => {
    const state = await tab.observe({ compact: true, viewportOnly: true });
    const node = state.elements.find(e => e.role === 'textbox' && e.name === name);
    if (!node) throw new Error(`Missing actual input: ${name}`);
    await tab.fill(`role/textbox[name=${JSON.stringify(node.name)}]`, value);
  };
  const passwordGone = () => page.waitForFunction(() => !document.querySelector('input[type="password"]'), { timeout: 20000 });
  await fill('Passphrase', passphrase);
  await click('Sign in');
  await passwordGone();
  await tab.pushState('/settings/devices');
  await tab.waitForText('ADD A PASSKEY FOR THIS DEVICE', { timeout: 15000 });
  const cdp = await page.target().createCDPSession();
  await cdp.send('WebAuthn.enable');
  const options = { protocol: 'ctap2', transport: 'internal', hasResidentKey: true, hasUserVerification: true, isUserVerified: true, automaticPresenceSimulation: true };
  const { authenticatorId } = await cdp.send('WebAuthn.addVirtualAuthenticator', { options });
  const label = 'PR4009 navigation-safe virtual';
  await fill('Passkey label', label);
  await click('ADD A PASSKEY FOR THIS DEVICE');
  await tab.waitForSelector('role/textbox[name="Enter passphrase"]', { timeout: 10000 });
  await fill('Enter passphrase', passphrase);
  await click('Confirm');
  await passwordGone();
  await click('ADD A PASSKEY FOR THIS DEVICE');
  await tab.waitForText(label, { timeout: 20000 });
  const { credentials } = await cdp.send('WebAuthn.getCredentials', { authenticatorId });
  if (credentials.length !== 1) throw new Error('Expected the actually enrolled virtual credential');
  await click('More options');
  await click('Sign out');
  await tab.waitForText('Sign in with passkey', { timeout: 15000 });
  // Chromium drops the virtual environment on this document navigation.
  await cdp.send('WebAuthn.enable');
  try { await cdp.send('WebAuthn.removeVirtualAuthenticator', { authenticatorId }); } catch {}
  const restored = await cdp.send('WebAuthn.addVirtualAuthenticator', { options });
  for (const credential of credentials) await cdp.send('WebAuthn.addCredential', { authenticatorId: restored.authenticatorId, credential });
  await page.bringToFront();
  await click('Sign in with passkey');
  await passwordGone();
  await tab.pushState('/settings/devices');
  await tab.waitForText(label, { timeout: 15000 });
  await click('Remove');
  await tab.waitForSelector('role/textbox[name="Enter passphrase"]', { timeout: 10000 });
  await click('Cancel');
  return { enrolled: true, passwordSessionSignedOut: true, credentialRestoredAfterNavigation: true, passkeySignIn: true, passkeyDoesNotElevate: true, authenticator: 'Chromium virtual platform, not physical hardware', label };
}
