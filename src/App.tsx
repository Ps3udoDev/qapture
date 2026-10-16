import logo from "./assets/brand/t4s-logo.png";

export default function App() {
  return (
    <main className="grid h-full place-items-center">
      <div className="text-center">
        <img src={logo} alt="Teams4Soft" className="mx-auto h-28" />
        <h1 className="mt-4 text-2xl font-semibold">Qapture</h1>
        <p className="text-t4s-gray">Evidencias QA · Teams4Soft</p>
      </div>
    </main>
  );
}
