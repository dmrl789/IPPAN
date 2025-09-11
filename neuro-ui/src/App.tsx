import { NavLink, Route, Routes } from "react-router-dom";
import ModelsPage from "./pages/ModelsPage";
import DatasetsPage from "./pages/DatasetsPage";
import InferencePage from "./pages/InferencePage";
import BidsPage from "./pages/BidsPage";
import ProofsPage from "./pages/ProofsPage";
import InteroperabilityPage from "./pages/InteroperabilityPage";

export default function App() {
  return (
    <div className="min-h-screen flex">
      <aside className="w-64 border-r p-4 space-y-3">
        <h1 className="text-xl font-bold">IPPAN • Neural UI</h1>
        <nav className="flex flex-col gap-1">
          {[
            ["Models", "/"],
            ["Datasets", "/datasets"],
            ["Post Inference", "/inference"],
            ["Bids / Winner", "/bids"],
            ["Proofs", "/proofs"],
            ["Interoperability", "/interoperability"],
          ].map(([label, to]) => (
            <NavLink key={to} to={to} className={({isActive}) =>
              `px-3 py-2 rounded ${isActive?"bg-black/10 dark:bg-white/10":"hover:bg-black/5 dark:hover:bg-white/5"}`
            }>{label}</NavLink>
          ))}
        </nav>
      </aside>
      <main className="flex-1 p-6">
        <Routes>
          <Route path="/" element={<ModelsPage />} />
          <Route path="/datasets" element={<DatasetsPage />} />
          <Route path="/inference" element={<InferencePage />} />
          <Route path="/bids" element={<BidsPage />} />
          <Route path="/proofs" element={<ProofsPage />} />
          <Route path="/interoperability" element={<InteroperabilityPage />} />
        </Routes>
      </main>
    </div>
  );
}
