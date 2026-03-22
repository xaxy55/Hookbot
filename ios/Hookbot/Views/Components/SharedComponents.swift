import SwiftUI

// MARK: - Stat Card

struct StatCardView: View {
    let value: String
    let label: String
    var color: Color = .cyan
    var icon: String? = nil

    var body: some View {
        VStack(spacing: 4) {
            if let icon {
                Image(systemName: icon)
                    .font(.system(size: 14))
                    .foregroundStyle(color)
            }
            Text(value)
                .font(.system(size: 18, weight: .bold, design: .monospaced))
                .foregroundStyle(color)
            Text(label)
                .font(.system(size: 10, weight: .medium, design: .monospaced))
                .foregroundStyle(.gray)
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical, 12)
        .background(RoundedRectangle(cornerRadius: 12).fill(Color(white: 0.08)))
    }
}

// MARK: - Loading State

struct LoadingStateView: View {
    var message: String = "Loading..."

    var body: some View {
        VStack(spacing: 12) {
            ProgressView()
                .tint(.cyan)
            Text(message)
                .font(.system(size: 12, design: .monospaced))
                .foregroundStyle(.gray)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .padding(.top, 60)
    }
}

// MARK: - Error State

struct ErrorStateView: View {
    let message: String
    var retry: (() -> Void)? = nil

    var body: some View {
        VStack(spacing: 16) {
            Image(systemName: "exclamationmark.triangle")
                .font(.system(size: 32))
                .foregroundStyle(.orange)
            Text(message)
                .font(.system(size: 13, design: .monospaced))
                .foregroundStyle(.gray)
                .multilineTextAlignment(.center)
            if let retry {
                Button("Retry") { retry() }
                    .font(.system(size: 14, weight: .semibold, design: .monospaced))
                    .foregroundStyle(.cyan)
            }
        }
        .padding(32)
    }
}

// MARK: - Empty State

struct EmptyStateView: View {
    let icon: String
    let message: String

    var body: some View {
        VStack(spacing: 12) {
            Image(systemName: icon)
                .font(.system(size: 36))
                .foregroundStyle(Color(white: 0.3))
            Text(message)
                .font(.system(size: 13, design: .monospaced))
                .foregroundStyle(.gray)
                .multilineTextAlignment(.center)
        }
        .padding(.top, 60)
    }
}

// MARK: - Category Filter Bar

struct CategoryFilterBar: View {
    let categories: [String]
    @Binding var selected: String

    var body: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: 8) {
                ForEach(categories, id: \.self) { cat in
                    Button {
                        selected = cat
                    } label: {
                        Text(cat)
                            .font(.system(size: 12, weight: .semibold, design: .monospaced))
                            .foregroundStyle(selected == cat ? .black : .white)
                            .padding(.horizontal, 14)
                            .padding(.vertical, 7)
                            .background(
                                Capsule().fill(selected == cat ? Color.cyan : Color(white: 0.15))
                            )
                    }
                }
            }
            .padding(.horizontal)
        }
    }
}

// MARK: - Section Header

struct SectionHeaderView: View {
    let title: String
    var icon: String? = nil

    var body: some View {
        HStack(spacing: 6) {
            if let icon {
                Image(systemName: icon)
                    .font(.system(size: 12))
                    .foregroundStyle(.cyan)
            }
            Text(title.uppercased())
                .font(.system(size: 11, weight: .bold, design: .monospaced))
                .foregroundStyle(.gray)
            Spacer()
        }
        .padding(.horizontal)
        .padding(.top, 8)
    }
}
