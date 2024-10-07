const Dummy = {
	Secp256k1PublicKey: Buffer.concat([
		Buffer.from([0x02]),
		Buffer.allocUnsafe(32),
	]).toString('base64'),
};

export default Dummy;
