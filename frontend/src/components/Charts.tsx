/**
 * ダッシュボードのグラフ（F-09 日別の推移、F-10 カテゴリ別の正答率）。
 *
 * 色は index.css の --series-* / --track を使い、ライト・ダーク両方で検証済み。
 * 色だけで意味を伝えないよう、凡例・ツールチップ・表表示を用意する。
 */

import { useState } from 'react'

import type { AreaStat, DailyCount } from '../api/types'
import { formatRate, formatShortDate, rateValue } from '../utils/format'

/** グラフの描画領域の高さ（px）。 */
const PLOT_HEIGHT = 160

/** 目盛りの最大値をきりのよい数にする。 */
function niceMax(value: number): number {
  if (value <= 4) return 4
  const magnitude = 10 ** Math.floor(Math.log10(value))
  for (const step of [1, 2, 2.5, 5, 10]) {
    const candidate = step * magnitude
    if (candidate >= value) return candidate
  }
  return 10 * magnitude
}

/** ツールチップがグラフの左右からはみ出さないよう、端の列では寄せる向きを変える。 */
function tooltipAlign(index: number, count: number): string {
  if (index < 2) return 'align-start'
  if (index > count - 3) return 'align-end'
  return ''
}

/** 日別の推移（正解・不正解の積み上げ縦棒）。 */
export function DailyChart({ data }: { data: DailyCount[] }) {
  const [hover, setHover] = useState<number | null>(null)
  const [showTable, setShowTable] = useState(false)
  const max = niceMax(Math.max(0, ...data.map((d) => d.count)))
  const ticks = [0, max / 2, max]
  const hovered = hover === null ? null : data[hover]

  return (
    <figure className="chart">
      <div className="chart-head">
        <figcaption>
          <h2>日別の推移</h2>
          <p className="muted">過去14日間の採点済み回答数</p>
        </figcaption>
        <button type="button" className="link-button" onClick={() => setShowTable((v) => !v)}>
          {showTable ? 'グラフで表示' : '表で表示'}
        </button>
      </div>

      <ul className="legend" aria-label="凡例">
        <li>
          <span className="swatch swatch-1" aria-hidden="true" />
          正解
        </li>
        <li>
          <span className="swatch swatch-2" aria-hidden="true" />
          不正解
        </li>
      </ul>

      {showTable ? (
        <table className="table compact">
          <thead>
            <tr>
              <th>日付</th>
              <th className="num">解答数</th>
              <th className="num">正解</th>
              <th className="num">不正解</th>
            </tr>
          </thead>
          <tbody>
            {data.map((d) => (
              <tr key={d.date}>
                <td>{d.date}</td>
                <td className="num">{d.count}</td>
                <td className="num">{d.correctCount}</td>
                <td className="num">{d.count - d.correctCount}</td>
              </tr>
            ))}
          </tbody>
        </table>
      ) : (
        <div className="daily-chart" onMouseLeave={() => setHover(null)}>
          <div className="y-axis" style={{ height: PLOT_HEIGHT }} aria-hidden="true">
            {[...ticks].reverse().map((t) => (
              <span key={t}>{t}</span>
            ))}
          </div>
          <div className="plot" style={{ height: PLOT_HEIGHT }}>
            {ticks.map((t) => (
              <div key={t} className="gridline" style={{ bottom: `${(t / max) * 100}%` }} aria-hidden="true" />
            ))}
            <div className="columns" role="list">
              {data.map((d, i) => {
                const wrong = d.count - d.correctCount
                return (
                  <div
                    key={d.date}
                    role="listitem"
                    className={`column-slot${hover === i ? ' is-hover' : ''}`}
                    onMouseEnter={() => setHover(i)}
                    onFocus={() => setHover(i)}
                    onBlur={() => setHover(null)}
                    tabIndex={0}
                    aria-label={`${d.date} 解答${d.count}問 正解${d.correctCount}問`}
                  >
                    <div className="column">
                      {wrong > 0 && (
                        <div className="segment segment-2" style={{ height: `${(wrong / max) * PLOT_HEIGHT}px` }} />
                      )}
                      {d.correctCount > 0 && (
                        <div
                          className="segment segment-1"
                          style={{ height: `${(d.correctCount / max) * PLOT_HEIGHT}px` }}
                        />
                      )}
                    </div>
                  </div>
                )
              })}
            </div>
            {hovered && hover !== null && (
              <div
                className={`tooltip ${tooltipAlign(hover, data.length)}`}
                style={{ left: `${((hover + 0.5) / data.length) * 100}%` }}
                role="status"
              >
                <strong>{hovered.date}</strong>
                <span>解答 {hovered.count}問</span>
                <span>
                  <span className="swatch swatch-1" aria-hidden="true" /> 正解 {hovered.correctCount}
                </span>
                <span>
                  <span className="swatch swatch-2" aria-hidden="true" /> 不正解 {hovered.count - hovered.correctCount}
                </span>
              </div>
            )}
          </div>
          <div className="x-axis" aria-hidden="true">
            {data.map((d, i) => (
              // 今日（右端）から 1 日おきに日付を表示する。
              <span key={d.date}>{(data.length - 1 - i) % 2 === 0 ? formatShortDate(d.date) : ''}</span>
            ))}
          </div>
        </div>
      )}
    </figure>
  )
}

/** カテゴリ別の正答率（横棒）。カテゴリを選ぶとそのカテゴリの履歴へ移動する。 */
export function AreaChart({
  data,
  onSelect,
}: {
  data: AreaStat[]
  onSelect: (area: AreaStat) => void
}) {
  return (
    <figure className="chart">
      <div className="chart-head">
        <figcaption>
          <h2>カテゴリ別の正答率</h2>
          <p className="muted">カテゴリを選ぶと、そのカテゴリの問題を表示します</p>
        </figcaption>
      </div>
      {data.length === 0 ? (
        <p className="empty">カテゴリがありません。</p>
      ) : (
        <ul className="area-bars">
          {data.map((stat) => {
            const rate = rateValue(stat.correctCount, stat.totalCount)
            return (
              <li key={stat.area}>
                <button
                  type="button"
                  className="area-row"
                  onClick={() => onSelect(stat)}
                  disabled={!stat.inMaster}
                  title={`${stat.area}：${formatRate(stat.correctCount, stat.totalCount)}（${stat.correctCount}/${stat.totalCount}）`}
                >
                  <span className="area-name">{stat.area}</span>
                  <span className="bar-track" aria-hidden="true">
                    {rate > 0 && <span className="bar-fill" style={{ width: `${rate}%` }} />}
                  </span>
                  <span className="area-value">
                    {formatRate(stat.correctCount, stat.totalCount)}
                    <span className="muted">
                      {' '}
                      {stat.correctCount}/{stat.totalCount}
                    </span>
                  </span>
                </button>
              </li>
            )
          })}
        </ul>
      )}
    </figure>
  )
}
