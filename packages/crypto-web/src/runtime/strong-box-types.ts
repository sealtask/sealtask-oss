export type StrongBoxEncryptInput = {
  key: Uint8Array
  context: Uint8Array
  plaintext: Uint8Array
}

export type StrongBoxDecryptInput = {
  key: Uint8Array
  context: Uint8Array
  ciphertext: Uint8Array
}

export type HpkeEncapInput = {
  recipientPublicKey: Uint8Array
  info: Uint8Array
  aad: Uint8Array
  plaintext: Uint8Array
}

export type HpkeEncapResult = {
  enc: Uint8Array
  nonce: Uint8Array
  ciphertext: Uint8Array
}

export type HpkeDecapInput = {
  recipientPrivateKey: Uint8Array
  info: Uint8Array
  aad: Uint8Array
  enc: Uint8Array
  ciphertext: Uint8Array
}

export interface StrongBoxBridge {
  encrypt(input: StrongBoxEncryptInput): Promise<Uint8Array>
  decrypt(input: StrongBoxDecryptInput): Promise<Uint8Array>
  hpkeEncap?(input: HpkeEncapInput): Promise<HpkeEncapResult>
  hpkeDecap?(input: HpkeDecapInput): Promise<Uint8Array>
}
