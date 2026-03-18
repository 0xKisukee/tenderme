"use client";

import { useEffect, useRef, useState } from "react";

interface ApprovalEvent {
  tx_hash: string;
  block_number: number;
  token_ticker: string;
  token_address: string;
  owner: string;
  spender: string;
  value: string;
  deployment_block: number;
}

type ConnectionStatus = "connecting" | "connected" | "disconnected";

const WS_URL = "ws://localhost:3001/ws";

const TOKEN_COLORS: Record<string, string> = {
  WETH: "bg-blue-500/20 text-blue-300 border-blue-500/30",
  USDC: "bg-green-500/20 text-green-300 border-green-500/30",
  USDT: "bg-emerald-500/20 text-emerald-300 border-emerald-500/30",
  DAI: "bg-yellow-500/20 text-yellow-300 border-yellow-500/30",
  stETH: "bg-cyan-500/20 text-cyan-300 border-cyan-500/30",
  BNB: "bg-amber-500/20 text-amber-300 border-amber-500/30",
  SOL: "bg-purple-500/20 text-purple-300 border-purple-500/30",
  LINK: "bg-indigo-500/20 text-indigo-300 border-indigo-500/30",
  WBTC: "bg-orange-500/20 text-orange-300 border-orange-500/30",
};

function shortAddr(addr: string) {
  return `${addr.slice(0, 6)}...${addr.slice(-4)}`;
}

function etherscanTx(hash: string) {
  return `https://etherscan.io/tx/${hash}`;
}

function etherscanAddr(addr: string) {
  return `https://etherscan.io/address/${addr}`;
}

function etherscanContract(addr: string) {
  return `https://etherscan.io/address/${addr}#code`;
}

function contractAgeDays(blockNumber: number, deploymentBlock: number): string {
  const blocks = blockNumber - deploymentBlock;
  const days = Math.floor(blocks / 7200);
  if (days === 0) return "< 1 day old";
  return `${days}d old`;
}

export default function Home() {
  const [events, setEvents] = useState<ApprovalEvent[]>([]);
  const [status, setStatus] = useState<ConnectionStatus>("connecting");
  const wsRef = useRef<WebSocket | null>(null);

  useEffect(() => {
    function connect() {
      const ws = new WebSocket(WS_URL);
      wsRef.current = ws;

      ws.onopen = () => setStatus("connected");

      ws.onmessage = (e) => {
        try {
          const event: ApprovalEvent = JSON.parse(e.data);
          setEvents((prev) => [event, ...prev].slice(0, 100));
        } catch {}
      };

      ws.onclose = () => {
        setStatus("disconnected");
        setTimeout(connect, 3000);
      };

      ws.onerror = () => {
        setStatus("disconnected");
        ws.close();
      };
    }

    connect();
    return () => wsRef.current?.close();
  }, []);

  const statusDot =
    status === "connected"
      ? "bg-green-400"
      : status === "connecting"
      ? "bg-yellow-400 animate-pulse"
      : "bg-red-400";

  const statusText =
    status === "connected"
      ? "Live"
      : status === "connecting"
      ? "Connecting..."
      : "Disconnected – retrying";

  return (
    <main className="min-h-screen bg-gray-950 text-gray-100 font-mono p-6">
      <div className="max-w-5xl mx-auto">
        {/* Header */}
        <div className="flex items-center justify-between mb-8">
          <div>
            <h1 className="text-2xl font-bold text-white tracking-tight">
              🚨 TenderMe
            </h1>
            <p className="text-gray-500 text-sm mt-1">
              Real-time ERC20 contract approval monitor
            </p>
          </div>
          <div className="flex items-center gap-2 bg-gray-900 border border-gray-800 rounded-lg px-4 py-2">
            <span className={`w-2 h-2 rounded-full ${statusDot}`} />
            <span className="text-sm text-gray-300">{statusText}</span>
            {events.length > 0 && (
              <span className="ml-3 text-xs text-gray-500">
                {events.length} event{events.length !== 1 ? "s" : ""}
              </span>
            )}
          </div>
        </div>

        {/* Empty state */}
        {events.length === 0 && (
          <div className="flex flex-col items-center justify-center border border-dashed border-gray-800 rounded-xl py-24 text-gray-600">
            <div className="text-4xl mb-4">👀</div>
            <p className="text-sm">Watching for suspicious approvals…</p>
          </div>
        )}

        {/* Event feed */}
        <div className="space-y-3">
          {events.map((ev, i) => (
            <div
              key={`${ev.tx_hash}-${i}`}
              className="bg-gray-900 border border-gray-800 rounded-xl p-4 hover:border-gray-700 transition-colors"
            >
              {/* Top row */}
              <div className="flex items-center justify-between mb-3">
                <div className="flex items-center gap-2">
                  <span
                    className={`text-xs font-semibold px-2 py-0.5 rounded border ${
                      TOKEN_COLORS[ev.token_ticker] ??
                      "bg-gray-700/40 text-gray-300 border-gray-600"
                    }`}
                  >
                    {ev.token_ticker}
                  </span>
                  <span className="text-red-400 text-xs font-medium">
                    CONTRACT APPROVAL
                  </span>
                  <span className="text-xs px-2 py-0.5 rounded border bg-orange-500/10 text-orange-400 border-orange-800">
                    🆕 {contractAgeDays(ev.block_number, ev.deployment_block)}
                  </span>
                </div>
                <span className="text-xs text-gray-600">
                  block {ev.block_number.toLocaleString()}
                </span>
              </div>

              {/* Details grid */}
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-2 text-xs">
                <div>
                  <span className="text-gray-600">Tx Hash</span>
                  <a
                    href={etherscanTx(ev.tx_hash)}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="block text-blue-400 hover:text-blue-300 truncate"
                  >
                    {ev.tx_hash}
                  </a>
                </div>

                <div>
                  <span className="text-gray-600">Amount</span>
                  <p className={`truncate ${ev.value === "Unlimited" ? "text-red-400 font-bold" : "text-yellow-300"}`}>
                    {ev.value} {ev.token_ticker}
                  </p>
                </div>

                <div>
                  <span className="text-gray-600">Owner</span>
                  <a
                    href={etherscanAddr(ev.owner)}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="block text-gray-300 hover:text-white"
                  >
                    {shortAddr(ev.owner)}
                  </a>
                </div>

                <div>
                  <span className="text-gray-600">Spender Contract</span>
                  <a
                    href={etherscanContract(ev.spender)}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="block text-red-400 hover:text-red-300"
                  >
                    {shortAddr(ev.spender)}
                  </a>
                </div>
              </div>
            </div>
          ))}
        </div>
      </div>
    </main>
  );
}
