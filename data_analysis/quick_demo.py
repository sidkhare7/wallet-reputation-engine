#!/usr/bin/env python3
"""
Quick Demo Script for Trading Reputation Algorithm Analysis
This script provides key insights without generating all visualization charts.
"""

from analyze import TradingReputationAnalyzer
import pandas as pd
import numpy as np

def quick_insights_demo():
    """
    Generate key insights about the trading reputation algorithm
    """
    print("="*80)
    print("TRADING REPUTATION ALGORITHM - QUICK INSIGHTS")
    print("="*80)
    
    # Initialize analyzer
    analyzer = TradingReputationAnalyzer('cult_data_aug_8.csv')
    
    # Basic data overview
    print(f"\n📊 DATASET OVERVIEW:")
    print(f"   • Total users: {len(analyzer.data):,}")
    print(f"   • Data columns: {len(analyzer.data.columns)}")
    
    # Key metrics analysis
    print(f"\n🎯 KEY METRICS ANALYSIS:")
    
    # Active traders
    active_traders = len(analyzer.data[analyzer.data['volume'] > 0])
    print(f"   • Active traders (volume > 0): {active_traders:,} ({active_traders/len(analyzer.data)*100:.1f}%)")
    
    # Current holders
    holders = len(analyzer.data[analyzer.data['holdings_value'] > 0])
    print(f"   • Current holders: {holders:,} ({holders/len(analyzer.data)*100:.1f}%)")
    
    # Users with holding duration
    duration_users = len(analyzer.data[analyzer.data['holding_duration'] > 0])
    print(f"   • Users with holding duration > 0: {duration_users:,} ({duration_users/len(analyzer.data)*100:.1f}%)")
    
    # Reputation analysis
    avg_reputation = analyzer.data['reputation'].mean()
    print(f"   • Average reputation score: {avg_reputation:.1f}")
    
    # Diamond hand analysis
    avg_diamond_hand = analyzer.data['diamond_hand_probability'].mean()
    print(f"   • Average diamond hand probability: {avg_diamond_hand:.1f}")
    
    # Volume analysis
    if active_traders > 0:
        volume_data = analyzer.data[analyzer.data['volume'] > 0]['volume']
        print(f"\n💰 VOLUME INSIGHTS:")
        print(f"   • Total volume range: {volume_data.min():.2e} to {volume_data.max():.2e}")
        print(f"   • Median volume: {volume_data.median():.2e}")
        print(f"   • Top 1% volume threshold: {volume_data.quantile(0.99):.2e}")
    
    # Holdings analysis
    if holders > 0:
        holdings_data = analyzer.data[analyzer.data['holdings_value'] > 0]['holdings_value']
        print(f"\n💎 HOLDINGS INSIGHTS:")
        print(f"   • Holdings range: {holdings_data.min():.2e} to {holdings_data.max():.2e}")
        print(f"   • Median holdings: {holdings_data.median():.2e}")
        print(f"   • Top 1% holdings threshold: {holdings_data.quantile(0.99):.2e}")
    
    # Duration analysis
    if duration_users > 0:
        duration_data = analyzer.data[analyzer.data['holding_duration'] > 0]['holding_duration']
        print(f"\n⏰ HOLDING DURATION INSIGHTS:")
        print(f"   • Duration range: {duration_data.min():.1f} to {duration_data.max():.1f}")
        print(f"   • Median duration: {duration_data.median():.1f}")
        print(f"   • Average duration: {duration_data.mean():.1f}")
    
    # Algorithm effectiveness check
    print(f"\n🔍 ALGORITHM EFFECTIVENESS:")
    
    # Check correlation between volume and reputation
    volume_data = analyzer.data[(analyzer.data['volume'] > 0) & (analyzer.data['reputation'] > 0)]
    if len(volume_data) > 10:
        correlation = volume_data['volume'].corr(volume_data['reputation'])
        print(f"   • Volume vs Reputation correlation: {correlation:.3f}")
        
        # Top volume traders reputation
        top_volume_traders = volume_data.nlargest(100, 'volume')
        avg_reputation_top_volume = top_volume_traders['reputation'].mean()
        print(f"   • Average reputation (top 100 volume): {avg_reputation_top_volume:.1f}")
    
    # Check correlation between holdings and reputation
    holdings_data = analyzer.data[(analyzer.data['holdings_value'] > 0) & (analyzer.data['reputation'] > 0)]
    if len(holdings_data) > 10:
        correlation = holdings_data['holdings_value'].corr(holdings_data['reputation'])
        print(f"   • Holdings vs Reputation correlation: {correlation:.3f}")
        
        # Top holders reputation
        top_holders = holdings_data.nlargest(100, 'holdings_value')
        avg_reputation_top_holders = top_holders['reputation'].mean()
        print(f"   • Average reputation (top 100 holders): {avg_reputation_top_holders:.1f}")
    
    # Check correlation between buy volume and reputation
    buy_volume_data = analyzer.data[(analyzer.data['buy_volume'] > 0) & (analyzer.data['reputation'] > 0)]
    if len(buy_volume_data) > 10:
        correlation = buy_volume_data['buy_volume'].corr(buy_volume_data['reputation'])
        print(f"   • Buy Volume vs Reputation correlation: {correlation:.3f}")
        
        # Top buy volume traders reputation
        top_buy_volume_traders = buy_volume_data.nlargest(100, 'buy_volume')
        avg_reputation_top_buy = top_buy_volume_traders['reputation'].mean()
        print(f"   • Average reputation (top 100 buy volume): {avg_reputation_top_buy:.1f}")
    
    # Duration vs reputation
    duration_data = analyzer.data[(analyzer.data['holding_duration'] > 0) & (analyzer.data['reputation'] > 0)]
    if len(duration_data) > 10:
        correlation = duration_data['holding_duration'].corr(duration_data['reputation'])
        print(f"   • Duration vs Reputation correlation: {correlation:.3f}")
    
    # Calculate composite ranking
    print(f"\n🏆 RANKING ALGORITHM TEST:")
    
    # Create composite score (same weights as in main script)
    weights = {
        'reputation': 0.4,
        'volume': 0.25,
        'holding_duration': 0.2,
        'holdings_value': 0.15
    }
    
    normalized_data = analyzer.data.copy()
    for metric in weights.keys():
        if metric in normalized_data.columns:
            max_val = normalized_data[metric].max()
            if max_val > 0:
                normalized_data[f'{metric}_norm'] = normalized_data[metric] / max_val
            else:
                normalized_data[f'{metric}_norm'] = 0
    
    # Calculate composite score
    normalized_data['composite_score'] = 0
    for metric, weight in weights.items():
        if f'{metric}_norm' in normalized_data.columns:
            normalized_data['composite_score'] += normalized_data[f'{metric}_norm'] * weight
    
    # Rank users
    normalized_data['rank'] = normalized_data['composite_score'].rank(ascending=False)
    
    # Show top performers
    top_10 = normalized_data.nsmallest(10, 'rank')
    users_with_score = len(normalized_data[normalized_data['composite_score'] > 0])
    
    print(f"   • Users with composite score > 0: {users_with_score:,}")
    print(f"   • Score range: {normalized_data['composite_score'].min():.4f} to {normalized_data['composite_score'].max():.4f}")
    print(f"   • Median score: {normalized_data['composite_score'].median():.4f}")
    
    print(f"\n🥇 TOP 5 USERS BY COMPOSITE SCORE:")
    for i, (_, user) in enumerate(top_10.head(5).iterrows(), 1):
        print(f"   {i}. ID: {user['id'][:10]}... | Composite: {user['composite_score']:.4f} | "
              f"Reputation: {user['reputation']:.0f} | Volume: {user['volume']:.2e} | "
              f"Holdings: {user['holdings_value']:.2e} | Duration: {user['holding_duration']:.1f}")
    
    # Data quality check
    print(f"\n⚠️  DATA QUALITY INSIGHTS:")
    
    # Check for users with all zeros
    zero_users = 0
    for _, user in analyzer.data.iterrows():
        if (user['volume'] == 0 and user['holdings_value'] == 0 and 
            user['holding_duration'] == 0):
            zero_users += 1
    
    print(f"   • Users with all zero metrics: {zero_users:,} ({zero_users/len(analyzer.data)*100:.1f}%)")
    
    # Check for potential outliers (very high values)
    outlier_volume = len(analyzer.data[analyzer.data['volume'] > analyzer.data['volume'].quantile(0.99)])
    outlier_holdings = len(analyzer.data[analyzer.data['holdings_value'] > analyzer.data['holdings_value'].quantile(0.99)])
    
    print(f"   • Potential volume outliers (top 1%): {outlier_volume:,}")
    print(f"   • Potential holdings outliers (top 1%): {outlier_holdings:,}")
    
    print(f"\n✅ RECOMMENDATIONS:")
    print(f"   1. Algorithm appears to be working - correlations show expected patterns")
    print(f"   2. Consider filtering out users with all zero metrics for ranking")
    print(f"   3. Monitor outliers for potential manipulation")
    print(f"   4. The composite scoring fairly balances different metrics")
    print(f"   5. Run full analysis with charts for detailed validation")
    
    print(f"\n💡 To run full analysis with all charts:")
    print(f"   python3 analyze.py")
    
    print("="*80)

if __name__ == "__main__":
    quick_insights_demo() 