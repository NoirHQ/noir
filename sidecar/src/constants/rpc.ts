const rpc = {
  cosm: {
    broadcastTx: {
      description: "Broadcast cosmos tx.",
      params: [
        {
          name: "tx_bytes",
          type: "Bytes",
        },
      ],
      type: "H256",
    },
  },
};

export default rpc;
