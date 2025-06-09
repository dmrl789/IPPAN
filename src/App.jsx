import { useEffect, useState } from "react";

export default function App() {
  const [accounts, setAccounts] = useState([]);
  const [domains, setDomains] = useState([]);
  const [loading, setLoading] = useState(false);

  // Form state
  const [newAccount, setNewAccount] = useState({ address: "", balance: "" });
  const [tx, setTx] = useState({ from: "", to: "", amount: "" });
  const [domainReg, setDomainReg] = useState({ from: "", domain: "", years: 1 });

  const [feedback, setFeedback] = useState("");

  const fetchAll = async () => {
    setLoading(true);
    setFeedback("");
    const accs = await fetch("http://localhost:3030/accounts").then(r => r.json());
    const doms = await fetch("http://localhost:3030/domains").then(r => r.json());
    setAccounts(accs);
    setDomains(doms);
    setLoading(false);
  };

  useEffect(() => { fetchAll(); }, []);

  // --------- Form handlers ----------
  async function handleAccountCreate(e) {
    e.preventDefault();
    setFeedback("");
    const res = await fetch("http://localhost:3030/account", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        address: newAccount.address,
        balance: parseInt(newAccount.balance || "0"),
      }),
    });
    setFeedback((await res.json()).status || (await res.json()).error);
    setNewAccount({ address: "", balance: "" });
    fetchAll();
  }

  async function handleSend(e) {
    e.preventDefault();
    setFeedback("");
    const res = await fetch("http://localhost:3030/transfer", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        from: tx.from,
        to: tx.to,
        amount: parseInt(tx.amount || "0"),
      }),
    });
    setFeedback((await res.json()).status || (await res.json()).error);
    setTx({ from: "", to: "", amount: "" });
    fetchAll();
  }

  async function handleDomainRegister(e) {
    e.preventDefault();
    setFeedback("");
    const res = await fetch("http://localhost:3030/register_domain", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        from: domainReg.from,
        domain: domainReg.domain,
        years: parseInt(domainReg.years || "1"),
      }),
    });
    setFeedback((await res.json()).status || (await res.json()).error);
    setDomainReg({ from: "", domain: "", years: 1 });
    fetchAll();
  }

  // ---------- Render ----------
  return (
    <div style={{ maxWidth: 900, margin: "0 auto", fontFamily: "sans-serif" }}>
      <h1>IPPAN Blockchain Dashboard</h1>

      <div style={{ color: "#31708f", fontWeight: "bold", marginBottom: 10 }}>{feedback}</div>

      {loading && <div>Loading...</div>}

      <div style={{ display: "flex", gap: 30, marginBottom: 32 }}>
        {/* Create account */}
        <form onSubmit={handleAccountCreate} style={{ flex: 1, background: "#f9f9f9", padding: 18, borderRadius: 8 }}>
          <h3>Create Account</h3>
          <input required
            placeholder="Address"
            value={newAccount.address}
            onChange={e => setNewAccount(a => ({ ...a, address: e.target.value }))}
            style={{ width: "100%", marginBottom: 5 }} />
          <input
            placeholder="Initial balance"
            type="number"
            value={newAccount.balance}
            onChange={e => setNewAccount(a => ({ ...a, balance: e.target.value }))}
            style={{ width: "100%", marginBottom: 5 }} />
          <button>Create</button>
        </form>

        {/* Send tokens */}
        <form onSubmit={handleSend} style={{ flex: 1, background: "#f9f9f9", padding: 18, borderRadius: 8 }}>
          <h3>Send IPN</h3>
          <input required
            placeholder="From address"
            value={tx.from}
            onChange={e => setTx(t => ({ ...t, from: e.target.value }))}
            style={{ width: "100%", marginBottom: 5 }} />
          <input required
            placeholder="To address or @handle"
            value={tx.to}
            onChange={e => setTx(t => ({ ...t, to: e.target.value }))}
            style={{ width: "100%", marginBottom: 5 }} />
          <input required
            placeholder="Amount"
            type="number"
            value={tx.amount}
            onChange={e => setTx(t => ({ ...t, amount: e.target.value }))}
            style={{ width: "100%", marginBottom: 5 }} />
          <button>Send</button>
        </form>

        {/* Register domain */}
        <form onSubmit={handleDomainRegister} style={{ flex: 1, background: "#f9f9f9", padding: 18, borderRadius: 8 }}>
          <h3>Register Domain</h3>
          <input required
            placeholder="Your address"
            value={domainReg.from}
            onChange={e => setDomainReg(d => ({ ...d, from: e.target.value }))}
            style={{ width: "100%", marginBottom: 5 }} />
          <input required
            placeholder="Domain (e.g. @alice.ipn)"
            value={domainReg.domain}
            onChange={e => setDomainReg(d => ({ ...d, domain: e.target.value }))}
            style={{ width: "100%", marginBottom: 5 }} />
          <input
            placeholder="Years"
            type="number"
            value={domainReg.years}
            onChange={e => setDomainReg(d => ({ ...d, years: e.target.value }))}
            style={{ width: "100%", marginBottom: 5 }} />
          <button>Register</button>
        </form>
      </div>

      <h2>Accounts</h2>
      <table border="1" cellPadding="6" style={{ width: "100%", marginBottom: 20 }}>
        <thead>
          <tr>
            <th>Address</th>
            <th>Balance</th>
            <th>Domains</th>
          </tr>
        </thead>
        <tbody>
          {accounts.map((acc) => (
            <tr key={acc.address}>
              <td>{acc.address}</td>
              <td>{acc.balance}</td>
              <td>
                {[...(acc.domains || [])].join(", ")}
              </td>
            </tr>
          ))}
        </tbody>
      </table>

      <h2>Domains</h2>
      <table border="1" cellPadding="6" style={{ width: "100%", marginBottom: 20 }}>
        <thead>
          <tr>
            <th>Name</th>
            <th>Owner</th>
            <th>Expires</th>
            <th>Target</th>
          </tr>
        </thead>
        <tbody>
          {domains.map((d) => (
            <tr key={d.name}>
              <td>{d.name}</td>
              <td>{d.owner}</td>
              <td>{d.expires}</td>
              <td>{d.target || ""}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
