import ReactECharts from "echarts-for-react";
import { useMemo } from "react";

interface Props {
  data: number[];
  color?: string;
  height?: number;
  max?: number;
}

export default function SparkChart({ data, color = "#2ec9a5", height = 32, max }: Props) {
  const option = useMemo(
    () => ({
      backgroundColor: "transparent",
      grid: { left: 0, right: 0, top: 0, bottom: 0 },
      xAxis: { type: "category" as const, show: false, data: data.map((_, i) => i) },
      yAxis: { type: "value" as const, show: false, min: 0, max: max ?? undefined },
      series: [
        {
          type: "line",
          data,
          showSymbol: false,
          smooth: true,
          lineStyle: { width: 2, color, shadowColor: color + "55", shadowBlur: 4 },
          areaStyle: {
            color: {
              type: "linear",
              x: 0,
              y: 0,
              x2: 0,
              y2: 1,
              colorStops: [
                { offset: 0, color: color + "30" },
                { offset: 1, color: color + "00" },
              ],
            },
          },
        },
      ],
      animation: false,
    }),
    [data, color, max],
  );

  if (data.length < 2) return null;

  return <ReactECharts option={option} style={{ height, width: "100%" }} opts={{ renderer: "svg" }} />;
}
