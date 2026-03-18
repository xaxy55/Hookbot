import SwiftUI

// Port of avatar.cpp drawFace() - renders the CEO avatar using SwiftUI Canvas
// Coordinate system: 128x64 logical (matching OLED), scaled up for display

struct AvatarRenderer {

    // MARK: - Draw the full avatar face

    static func draw(
        context: inout GraphicsContext,
        size: CGSize,
        params: AvatarParams,
        state: AvatarState,
        stateTime: Float,
        totalTime: Float,
        accessories: AccessoriesConfig,
        tool: ToolInfo,
        tasks: [TaskItem],
        activeTaskIndex: Int
    ) {
        // Scale from 128x64 logical to actual view size
        let scaleX = size.width / 128.0
        let scaleY = size.height / 64.0
        let scale = min(scaleX, scaleY)

        let offsetX = (size.width - 128 * scale) / 2
        let offsetY = (size.height - 64 * scale) / 2

        // Helper to convert logical coords to view coords
        func pt(_ x: CGFloat, _ y: CGFloat) -> CGPoint {
            CGPoint(x: offsetX + x * scale, y: offsetY + y * scale)
        }
        func sz(_ w: CGFloat, _ h: CGFloat) -> CGSize {
            CGSize(width: w * scale, height: h * scale)
        }
        func val(_ v: CGFloat) -> CGFloat { v * scale }

        let color = Color.white

        let hasTasks = !tasks.isEmpty
        let faceOffset: CGFloat = hasTasks ? -20 : 0

        let cx: CGFloat = 64 + CGFloat(params.shake) + faceOffset
        let cy: CGFloat = 32 + CGFloat(params.bounce)

        // ─── Top Hat ─────────
        if accessories.topHat {
            let hatBrimY = cy - 22
            let hatTopY = cy - 38
            let hatW: CGFloat = 14
            let brimW: CGFloat = 20

            // Brim (2px thick)
            context.fill(Path(CGRect(origin: pt(cx - brimW, hatBrimY), size: sz(brimW * 2 + 1, 2))), with: .color(color))
            // Sides
            context.fill(Path(CGRect(origin: pt(cx - hatW, hatTopY), size: sz(1, hatBrimY - hatTopY))), with: .color(color))
            context.fill(Path(CGRect(origin: pt(cx + hatW, hatTopY), size: sz(1, hatBrimY - hatTopY))), with: .color(color))
            // Top
            context.fill(Path(CGRect(origin: pt(cx - hatW, hatTopY), size: sz(hatW * 2 + 1, 1))), with: .color(color))
            // Band
            context.fill(Path(CGRect(origin: pt(cx - hatW + 1, hatBrimY - 4), size: sz(hatW * 2 - 1, 2))), with: .color(color))
        }

        // ─── Crown ─────────
        if accessories.crown {
            let crownY = cy - 24
            var crownPath = Path()
            crownPath.move(to: pt(cx - 16, crownY))
            crownPath.addLine(to: pt(cx - 16, crownY - 10))
            crownPath.addLine(to: pt(cx - 10, crownY - 5))
            crownPath.addLine(to: pt(cx - 4, crownY - 14))
            crownPath.addLine(to: pt(cx, crownY - 8))
            crownPath.addLine(to: pt(cx + 4, crownY - 14))
            crownPath.addLine(to: pt(cx + 10, crownY - 5))
            crownPath.addLine(to: pt(cx + 16, crownY - 10))
            crownPath.addLine(to: pt(cx + 16, crownY))
            context.stroke(crownPath, with: .color(color), lineWidth: val(1))
            // Jewels
            for jx in [cx, cx - 4, cx + 4] as [CGFloat] {
                let jy: CGFloat = jx == cx ? crownY - 8 : crownY - 14
                context.fill(Path(ellipseIn: CGRect(origin: pt(jx - 1, jy - 1), size: sz(3, 3))), with: .color(color))
            }
        }

        // ─── Devil Horns ─────────
        if accessories.horns {
            let hornY = cy - 22
            for side in [-1, 1] as [CGFloat] {
                let hx = cx + side * 12
                var hornPath = Path()
                hornPath.move(to: pt(hx, hornY))
                hornPath.addLine(to: pt(hx + side * 4, hornY - 8))
                hornPath.addLine(to: pt(hx + side * 2, hornY - 14))
                context.stroke(hornPath, with: .color(color), lineWidth: val(2))
            }
        }

        // ─── Halo ─────────
        if accessories.halo {
            let haloY = cy - 28
            let haloPath = Path(ellipseIn: CGRect(origin: pt(cx - 12, haloY - 3), size: sz(24, 6)))
            context.stroke(haloPath, with: .color(color), lineWidth: val(1))
        }

        // ─── Eyebrows ─────────
        let eyeSpacing: CGFloat = 18
        let browBaseY = cy - 18

        for side in [-1, 1] as [CGFloat] {
            let bx = cx + side * eyeSpacing
            let by = browBaseY + CGFloat(params.browY)
            let angle = CGFloat(params.browAngle)
            let innerY = by + angle * -3.0
            let outerY = by + angle * 3.0
            let innerX = bx - side * 2
            let outerX = bx + side * 8

            var browPath = Path()
            browPath.move(to: pt(innerX, innerY))
            browPath.addLine(to: pt(outerX, outerY))
            context.stroke(browPath, with: .color(color), lineWidth: val(2))
        }

        // ─── Eyes ─────────
        let eyeBaseY = cy - 8
        let eyeW: CGFloat = 10
        let eyeMaxH: CGFloat = 12

        let openness = max(0, min(CGFloat(params.eyeOpen), 1.2))
        var eyeH = eyeMaxH * openness
        if eyeH < 1 { eyeH = 1 }

        let pupilOffX = CGFloat(params.eyeX) * 3.0
        let pupilOffY = CGFloat(params.eyeY) * 2.0

        for side in [-1, 1] as [CGFloat] {
            let ex = cx + side * eyeSpacing
            let ey = eyeBaseY

            if eyeH <= 2 {
                // Closed eye
                context.fill(Path(CGRect(origin: pt(ex - eyeW / 2, ey), size: sz(eyeW, 1))), with: .color(color))
            } else {
                // Open eye - rounded rect
                let r = min(eyeW / 2, eyeH / 2)
                let eyeRect = CGRect(origin: pt(ex - eyeW / 2, ey - eyeH / 2), size: sz(eyeW, eyeH))
                context.fill(Path(roundedRect: eyeRect, cornerRadius: val(r)), with: .color(color))

                // Pupil (dark)
                if eyeH > 5 {
                    let px = ex + pupilOffX
                    let py = ey + pupilOffY
                    context.fill(
                        Path(ellipseIn: CGRect(origin: pt(px - 2, py - 2), size: sz(4, 4))),
                        with: .color(.black)
                    )
                }
            }
        }

        // ─── Glasses ─────────
        if accessories.glasses {
            for side in [-1, 1] as [CGFloat] {
                let ex = cx + side * eyeSpacing
                let lensRect = CGRect(origin: pt(ex - eyeW / 2 - 2, eyeBaseY - 7), size: sz(eyeW + 4, 14))
                context.stroke(Path(roundedRect: lensRect, cornerRadius: val(3)), with: .color(color), lineWidth: val(1))
            }
            // Bridge
            var bridge = Path()
            bridge.move(to: pt(cx - eyeSpacing + eyeW / 2 + 2, eyeBaseY))
            bridge.addLine(to: pt(cx + eyeSpacing - eyeW / 2 - 2, eyeBaseY))
            context.stroke(bridge, with: .color(color), lineWidth: val(1))
            // Arms
            var leftArm = Path()
            leftArm.move(to: pt(cx - eyeSpacing - eyeW / 2 - 2, eyeBaseY - 3))
            leftArm.addLine(to: pt(cx - eyeSpacing - eyeW / 2 - 8, eyeBaseY - 3))
            context.stroke(leftArm, with: .color(color), lineWidth: val(1))
            var rightArm = Path()
            rightArm.move(to: pt(cx + eyeSpacing + eyeW / 2 + 2, eyeBaseY - 3))
            rightArm.addLine(to: pt(cx + eyeSpacing + eyeW / 2 + 8, eyeBaseY - 3))
            context.stroke(rightArm, with: .color(color), lineWidth: val(1))
        }

        // ─── Monocle ─────────
        if accessories.monocle {
            let ex = cx + eyeSpacing
            let monocleR: CGFloat = eyeW / 2 + 3
            context.stroke(
                Path(ellipseIn: CGRect(origin: pt(ex - monocleR, eyeBaseY - monocleR), size: sz(monocleR * 2, monocleR * 2))),
                with: .color(color), lineWidth: val(2)
            )
            // Chain
            var chain = Path()
            let chainX = ex + eyeW / 2 + 2
            let chainStartY = eyeBaseY + eyeW / 2 + 2
            for i in 0..<12 {
                let py = chainStartY + CGFloat(i) * 2
                let px = chainX + CGFloat(sinf(Float(i) * 0.8)) * 2
                if py < 64 {
                    if i == 0 { chain.move(to: pt(px, py)) }
                    else { chain.addLine(to: pt(px, py)) }
                }
            }
            context.stroke(chain, with: .color(color), lineWidth: val(1))
        }

        // ─── Mouth ─────────
        let mouthY = cy + 12
        let mouthW: CGFloat = 16

        if params.mouthCurve > 0.1 {
            // Evil grin
            let curveH = CGFloat(params.mouthCurve) * 6.0
            let openH = CGFloat(params.mouthOpen) * 6.0

            if openH > 1 {
                let mouthRect = CGRect(origin: pt(cx - mouthW / 2, mouthY), size: sz(mouthW, openH + 2))
                context.fill(Path(roundedRect: mouthRect, cornerRadius: val(2)), with: .color(color))
            }
            // Grin curve
            var grin = Path()
            let steps = 20
            for i in 0...steps {
                let t = CGFloat(i) / CGFloat(steps) * 2.0 - 1.0
                let dx = t * mouthW / 2
                let dy = curveH * (1.0 - t * t)
                let p = pt(cx + dx, mouthY + dy)
                if i == 0 { grin.move(to: p) }
                else { grin.addLine(to: p) }
            }
            context.stroke(grin, with: .color(color), lineWidth: val(1))
        } else if params.mouthCurve < -0.1 {
            // Frown
            let curveH = CGFloat(-params.mouthCurve) * 5.0
            let openH = CGFloat(params.mouthOpen) * 5.0

            if openH > 1 {
                let mouthRect = CGRect(origin: pt(cx - mouthW / 2, mouthY - 2), size: sz(mouthW, openH + 2))
                context.fill(Path(roundedRect: mouthRect, cornerRadius: val(2)), with: .color(color))
            }
            var frown = Path()
            let steps = 20
            for i in 0...steps {
                let t = CGFloat(i) / CGFloat(steps) * 2.0 - 1.0
                let dx = t * mouthW / 2
                let dy = -(curveH * (1.0 - t * t))
                let p = pt(cx + dx, mouthY + dy)
                if i == 0 { frown.move(to: p) }
                else { frown.addLine(to: p) }
            }
            context.stroke(frown, with: .color(color), lineWidth: val(1))
        } else {
            // Neutral line
            context.fill(Path(CGRect(origin: pt(cx - mouthW / 3, mouthY), size: sz(mouthW * 2 / 3, 1))), with: .color(color))
        }

        // ─── Cigar ─────────
        if accessories.cigar {
            let cigarX = cx + mouthW / 2 + 1
            let cigarY = mouthY + 1

            // Body
            var cigar = Path()
            cigar.move(to: pt(cigarX, cigarY))
            cigar.addLine(to: pt(cigarX + 10, cigarY - 3))
            context.stroke(cigar, with: .color(color), lineWidth: val(3))

            // Ember
            let flicker = sinf(totalTime / 150.0)
            if flicker > -0.3 {
                context.fill(
                    Path(ellipseIn: CGRect(origin: pt(cigarX + 9, cigarY - 4), size: sz(3, 3))),
                    with: .color(Color.orange)
                )
            }

            // Smoke particles
            let smokePhase = totalTime / 1000.0
            let smokeSrcX = cigarX + 11
            let smokeSrcY = cigarY - 4

            for i in 0..<5 {
                let fi = Float(i)
                let pLife = (smokePhase * 1.2 + fi * 0.7).truncatingRemainder(dividingBy: 3.0)
                if pLife > 2.5 { continue }
                let rise = CGFloat(pLife) * 5.0
                let drift = CGFloat(sinf(pLife * 2.0 + fi * 1.5)) * (2.0 + CGFloat(pLife))
                let sx = smokeSrcX + drift
                let sy = smokeSrcY - rise
                if sy >= 0 && sy < 64 && sx >= 0 && sx < 128 {
                    let opacity = 1.0 - Double(pLife) / 2.5
                    context.fill(
                        Path(ellipseIn: CGRect(origin: pt(sx, sy), size: sz(pLife < 1.5 ? 2 : 1, 1))),
                        with: .color(color.opacity(opacity * 0.6))
                    )
                }
            }
        }

        // ─── Bow Tie ─────────
        if accessories.bowtie {
            let tieY = cy + 20
            var bowLeft = Path()
            bowLeft.move(to: pt(cx, tieY))
            bowLeft.addLine(to: pt(cx - 8, tieY - 4))
            bowLeft.addLine(to: pt(cx - 8, tieY + 4))
            bowLeft.closeSubpath()
            context.fill(bowLeft, with: .color(color))

            var bowRight = Path()
            bowRight.move(to: pt(cx, tieY))
            bowRight.addLine(to: pt(cx + 8, tieY - 4))
            bowRight.addLine(to: pt(cx + 8, tieY + 4))
            bowRight.closeSubpath()
            context.fill(bowRight, with: .color(color))

            context.fill(Path(ellipseIn: CGRect(origin: pt(cx - 2, tieY - 2), size: sz(4, 4))), with: .color(color))
        }

        // ─── Tool Display ─────────
        if (state == .thinking || state == .taskcheck) && !tool.name.isEmpty {
            let toolY: CGFloat = 56
            let textX: CGFloat = 14

            // Tool icon
            drawToolIcon(context: &context, tool: tool.name, x: pt(2, toolY).x, y: pt(0, toolY).y, scale: scale)

            // Tool name + detail
            let text = tool.detail.isEmpty ? tool.name : "\(tool.name) \(tool.detail)"
            context.draw(
                Text(text).font(.system(size: val(7), weight: .medium, design: .monospaced)).foregroundColor(color),
                at: pt(textX, toolY + 3),
                anchor: .leading
            )

            // Scanning line
            if state == .thinking {
                let scanPhase = stateTime / 300.0
                let scanPos = 2 + CGFloat(scanPhase.truncatingRemainder(dividingBy: 1.0)) * 124
                context.fill(Path(CGRect(origin: pt(scanPos - 2, 54), size: sz(4, 1))), with: .color(color))
            }
        } else if state == .thinking {
            // Plotting dots
            let phase = stateTime / 400.0
            for i in 0..<3 {
                let dotX = cx - 6 + CGFloat(i) * 6
                let dotY = cy + 24
                let anim = sinf(phase * .pi + Float(i) * 1.0)
                if anim > 0.3 {
                    context.fill(
                        Path(ellipseIn: CGRect(origin: pt(dotX - 1, dotY - CGFloat(anim) * 2 - 1), size: sz(3, 3))),
                        with: .color(color)
                    )
                }
            }
        }

        // ─── Checkmark (taskcheck) ─────────
        if state == .taskcheck && stateTime < 800 {
            let progress = min(1.0, stateTime / 600.0)
            let checkX = cx - 6
            let checkY = cy + 24
            var check = Path()
            if progress < 0.4 {
                let t = CGFloat(progress / 0.4)
                check.move(to: pt(checkX, checkY))
                check.addLine(to: pt(checkX + 4 * t, checkY + 4 * t))
            } else {
                let t = CGFloat((progress - 0.4) / 0.6)
                check.move(to: pt(checkX, checkY))
                check.addLine(to: pt(checkX + 4, checkY + 4))
                check.addLine(to: pt(checkX + 4 + 8 * t, checkY + 4 - 8 * t))
            }
            context.stroke(check, with: .color(Color.green), lineWidth: val(2))
        }

        // ─── Error X marks ─────────
        if state == .error && stateTime > 300 {
            for s in [-1, 1] as [CGFloat] {
                let xc = cx + s * 8
                let yc = cy + 24
                var xMark = Path()
                xMark.move(to: pt(xc - 3, yc - 3))
                xMark.addLine(to: pt(xc + 3, yc + 3))
                xMark.move(to: pt(xc + 3, yc - 3))
                xMark.addLine(to: pt(xc - 3, yc + 3))
                context.stroke(xMark, with: .color(Color.red), lineWidth: val(1.5))
            }
        }

        // ─── Sleeping Zzz ─────────
        if state == .idle && stateTime > 60000 {
            let zPhase = totalTime / 1500.0
            for i in 0..<3 {
                let fi = Float(i)
                let zLife = (zPhase + fi * 1.2).truncatingRemainder(dividingBy: 3.5)
                if zLife > 3.0 { continue }
                let rise = CGFloat(zLife) * 6.0
                let drift = CGFloat(zLife) * 3.0
                let zx = cx + 22 + drift
                let zy = cy - 10 - rise
                let zSize = CGFloat(2 + i)
                if zy >= 0 && zx < 128 - zSize {
                    let opacity = 1.0 - Double(zLife) / 3.0
                    context.draw(
                        Text("Z").font(.system(size: val(zSize * 3), weight: .bold, design: .monospaced))
                            .foregroundColor(color.opacity(opacity)),
                        at: pt(zx, zy),
                        anchor: .center
                    )
                }
            }
        }

        // ─── Waiting exclamation marks ─────────
        if state == .waiting && stateTime > 3000 {
            let rage = min(1.0, stateTime / 10000.0)
            let pulse = sinf(totalTime / 200.0)
            let marks = 1 + Int(rage * 2)
            for m in 0..<marks {
                let mPhase = pulse + Float(m) * 0.8
                let mBounce = CGFloat(sinf(mPhase * 3.0)) * 2.0
                for side in [-1, 1] as [CGFloat] {
                    let mx = cx + side * (30 + CGFloat(m) * 7)
                    let my = cy - 8 + mBounce
                    if mx > 2 && mx < 126 {
                        // ! mark
                        context.fill(Path(CGRect(origin: pt(mx, my - 4), size: sz(1, 6))), with: .color(color))
                        context.fill(Path(ellipseIn: CGRect(origin: pt(mx - 0.5, my + 3.5), size: sz(2, 2))), with: .color(color))
                    }
                }
            }
        }

        // ─── Task List ─────────
        if hasTasks {
            drawTaskList(context: &context, tasks: tasks, activeIndex: activeTaskIndex, pt: pt, sz: sz, val: val, color: color, totalTime: totalTime)
        }
    }

    // MARK: - Tool Icon

    private static func drawToolIcon(context: inout GraphicsContext, tool: String, x: CGFloat, y: CGFloat, scale: CGFloat) {
        func val(_ v: CGFloat) -> CGFloat { v * scale }
        let color = Color.white

        let iconSize = val(8)
        let cx = x + iconSize / 2
        let cy = y + iconSize / 2

        switch tool {
        case "Read":
            context.stroke(Path(ellipseIn: CGRect(x: cx - val(3), y: cy - val(3), width: val(6), height: val(6))),
                          with: .color(color), lineWidth: val(1))
            context.fill(Path(ellipseIn: CGRect(x: cx - val(1), y: cy - val(1), width: val(2), height: val(2))),
                        with: .color(color))
        case "Write", "Edit":
            var pencil = Path()
            pencil.move(to: CGPoint(x: x + val(1), y: y + val(6)))
            pencil.addLine(to: CGPoint(x: x + val(7), y: y))
            context.stroke(pencil, with: .color(color), lineWidth: val(2))
        case "Bash":
            var term = Path()
            term.move(to: CGPoint(x: x, y: y + val(1)))
            term.addLine(to: CGPoint(x: x + val(3), y: y + val(3)))
            term.addLine(to: CGPoint(x: x, y: y + val(5)))
            context.stroke(term, with: .color(color), lineWidth: val(1))
            context.fill(Path(CGRect(x: x + val(4), y: y + val(6), width: val(4), height: val(1))), with: .color(color))
        case "Grep", "Glob":
            context.stroke(Path(ellipseIn: CGRect(x: x + val(1), y: y + val(1), width: val(5), height: val(5))),
                          with: .color(color), lineWidth: val(1))
            var handle = Path()
            handle.move(to: CGPoint(x: x + val(5), y: y + val(5)))
            handle.addLine(to: CGPoint(x: x + val(8), y: y + val(7)))
            context.stroke(handle, with: .color(color), lineWidth: val(1.5))
        case "Agent":
            context.stroke(Path(CGRect(x: x + val(1), y: y + val(2), width: val(7), height: val(5))),
                          with: .color(color), lineWidth: val(1))
            context.fill(Path(ellipseIn: CGRect(x: x + val(2.5), y: y + val(3.5), width: val(1.5), height: val(1.5))), with: .color(color))
            context.fill(Path(ellipseIn: CGRect(x: x + val(5), y: y + val(3.5), width: val(1.5), height: val(1.5))), with: .color(color))
        default:
            // Gear icon
            context.stroke(Path(ellipseIn: CGRect(x: cx - val(2), y: cy - val(2), width: val(4), height: val(4))),
                          with: .color(color), lineWidth: val(1))
        }
    }

    // MARK: - Task List

    private static func drawTaskList(
        context: inout GraphicsContext,
        tasks: [TaskItem],
        activeIndex: Int,
        pt: (CGFloat, CGFloat) -> CGPoint,
        sz: (CGFloat, CGFloat) -> CGSize,
        val: (CGFloat) -> CGFloat,
        color: Color,
        totalTime: Float
    ) {
        let startX: CGFloat = 72
        let startY: CGFloat = 2
        let lineH: CGFloat = 9
        let maxVisible = min(6, tasks.count)

        var scrollOffset = 0
        if activeIndex >= maxVisible {
            scrollOffset = activeIndex - maxVisible + 1
        }

        for i in 0..<maxVisible {
            let idx = i + scrollOffset
            if idx >= tasks.count { break }

            let y = startY + CGFloat(i) * lineH
            let item = tasks[idx]

            if item.status == 2 {
                // Done: filled box with check
                context.fill(Path(CGRect(origin: pt(startX, y + 1), size: sz(7, 7))), with: .color(color))
            } else if item.status == 3 {
                // Failed: X box
                context.stroke(Path(CGRect(origin: pt(startX, y + 1), size: sz(7, 7))), with: .color(color), lineWidth: val(1))
                var xp = Path()
                xp.move(to: pt(startX + 1, y + 2))
                xp.addLine(to: pt(startX + 6, y + 7))
                xp.move(to: pt(startX + 6, y + 2))
                xp.addLine(to: pt(startX + 1, y + 7))
                context.stroke(xp, with: .color(color), lineWidth: val(1))
            } else if item.status == 1 || idx == activeIndex {
                // Active: blinking dot
                let blink = Int(totalTime) % 600 < 400
                if blink {
                    context.fill(Path(ellipseIn: CGRect(origin: pt(startX, y + 1), size: sz(7, 7))), with: .color(color))
                } else {
                    context.stroke(Path(ellipseIn: CGRect(origin: pt(startX, y + 1), size: sz(7, 7))), with: .color(color), lineWidth: val(1))
                }
            } else {
                // Pending: empty box
                context.stroke(Path(CGRect(origin: pt(startX, y + 1), size: sz(7, 7))), with: .color(color), lineWidth: val(1))
            }

            // Label
            let maxChars = Int((128 - startX - 10) / 6)
            let label = String(item.label.prefix(maxChars))
            context.draw(
                Text(label).font(.system(size: val(7), design: .monospaced)).foregroundColor(color),
                at: pt(startX + 10, y + 1),
                anchor: .topLeading
            )

            // Strikethrough for done
            if item.status == 2 {
                let textW = CGFloat(min(label.count * 6, Int(128 - startX - 10)))
                context.fill(Path(CGRect(origin: pt(startX + 10, y + 4), size: sz(textW, 1))), with: .color(color))
            }
        }
    }
}
