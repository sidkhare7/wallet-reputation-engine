import pandas as pd
import numpy as np
import matplotlib.pyplot as plt
import seaborn as sns
from scipy import stats
from typing import Optional, Tuple, List
import warnings
warnings.filterwarnings('ignore')

# Set style for better-looking plots
plt.style.use('seaborn-v0_8')
sns.set_palette("husl")

class TradingReputationAnalyzer:
    """
    A comprehensive analyzer for trading reputation algorithm validation
    """
    
    def __init__(self, csv_file_path: str):
        """
        Initialize the analyzer with CSV data
        
        Args:
            csv_file_path: Path to the CSV file containing trading data
        """
        self.csv_file = csv_file_path
        self.data = None
        self.load_data()
    
    def load_data(self):
        """Load and preprocess the CSV data"""
        try:
            # Read CSV with proper handling of large numbers
            self.data = pd.read_csv(self.csv_file)
            
            # Convert numeric columns
            numeric_cols = ['diamond_hand_probability', 'reputation', 'volume', 'holding_duration', 
                          'holdings_value', 'buy_volume', 'sell_volume', 
                          'buy_volume_z', 'sell_volume_z', 'duration_z', 'volume_z']
            
            for col in numeric_cols:
                if col in self.data.columns:
                    self.data[col] = pd.to_numeric(self.data[col], errors='coerce')
            
            # Convert timestamp
            if 'first_bought' in self.data.columns:
                self.data['first_bought'] = pd.to_datetime(self.data['first_bought'])
            
            # Clean data - remove rows with all zeros or nulls in key metrics
            key_metrics = ['volume', 'holding_duration', 'holdings_value']
            self.data = self.data.dropna(subset=['id'])
            
            print(f"Loaded {len(self.data)} records")
            print(f"Columns: {list(self.data.columns)}")
            
        except Exception as e:
            print(f"Error loading data: {e}")
            raise
    
    def data_overview(self):
        """Print comprehensive data overview"""
        print("="*80)
        print("DATA OVERVIEW")
        print("="*80)
        
        print(f"Dataset shape: {self.data.shape}")
        print(f"Memory usage: {self.data.memory_usage(deep=True).sum() / 1024**2:.2f} MB")
        
        print("\nColumn Info:")
        print(self.data.info())
        
        print("\nBasic Statistics:")
        print(self.data.describe())
        
        print("\nMissing Values:")
        missing = self.data.isnull().sum()
        print(missing[missing > 0])
        
        print("\nUnique Values:")
        for col in self.data.columns:
            unique_count = self.data[col].nunique()
            print(f"{col}: {unique_count}")
    
    def plot_reputation_distributions(self, figsize=(15, 12)):
        """
        Plot distribution of key reputation metrics
        """
        fig, axes = plt.subplots(3, 3, figsize=figsize)
        fig.suptitle('Trading Reputation Metrics Distributions', fontsize=16, y=1.02)
        
        metrics = ['reputation', 'diamond_hand_probability', 'volume', 'holding_duration', 
                  'holdings_value', 'buy_volume', 'sell_volume']
        
        for i, metric in enumerate(metrics):
            if metric in self.data.columns:
                row, col = i // 3, i % 3
                ax = axes[row, col]
                
                # Remove zeros and outliers for better visualization
                data_clean = self.data[self.data[metric] > 0][metric]
                
                if len(data_clean) > 0:
                    # Histogram
                    ax.hist(data_clean, bins=50, alpha=0.7, edgecolor='black')
                    ax.set_title(f'{metric.replace("_", " ").title()}')
                    ax.set_xlabel(metric)
                    ax.set_ylabel('Frequency')
                    
                    # Add statistics
                    mean_val = data_clean.mean()
                    median_val = data_clean.median()
                    ax.axvline(mean_val, color='red', linestyle='--', label=f'Mean: {mean_val:.2e}')
                    ax.axvline(median_val, color='green', linestyle='--', label=f'Median: {median_val:.2e}')
                    ax.legend()
                else:
                    ax.text(0.5, 0.5, 'No positive data', transform=ax.transAxes, ha='center')
                    ax.set_title(f'{metric} (No Data)')
        
        # Hide empty subplots
        for i in range(len(metrics), 9):
            row, col = i // 3, i % 3
            axes[row, col].set_visible(False)
        
        plt.tight_layout()
        plt.show()
    
    def plot_correlation_matrix(self, figsize=(12, 10)):
        """
        Plot correlation matrix of reputation metrics
        """
        numeric_cols = self.data.select_dtypes(include=[np.number]).columns
        correlation_matrix = self.data[numeric_cols].corr()
        
        plt.figure(figsize=figsize)
        mask = np.triu(np.ones_like(correlation_matrix, dtype=bool))
        
        sns.heatmap(correlation_matrix, mask=mask, annot=True, cmap='coolwarm', 
                   center=0, square=True, fmt='.2f')
        plt.title('Correlation Matrix of Trading Metrics', fontsize=16)
        plt.tight_layout()
        plt.show()
        
        # Print strongest correlations
        print("\nStrongest Correlations (|r| > 0.5):")
        for i in range(len(correlation_matrix.columns)):
            for j in range(i+1, len(correlation_matrix.columns)):
                corr_val = correlation_matrix.iloc[i, j]
                if abs(corr_val) > 0.5:
                    print(f"{correlation_matrix.columns[i]} vs {correlation_matrix.columns[j]}: {corr_val:.3f}")
    
    def plot_reputation_impact_analysis(self, figsize=(20, 12)):
        """
        Analyze how holding_value and buy_volume specifically affect reputation
        """
        fig, axes = plt.subplots(2, 3, figsize=figsize)
        fig.suptitle('Reputation Impact Analysis: Holdings Value & Buy Volume Effects', fontsize=16)
        
        # 1. Holdings Value vs Reputation Scatter
        holdings_rep_data = self.data[(self.data['holdings_value'] > 0) & (self.data['reputation'] > 0)]
        if len(holdings_rep_data) > 0:
            scatter = axes[0, 0].scatter(holdings_rep_data['holdings_value'], holdings_rep_data['reputation'], 
                                       alpha=0.6, c=holdings_rep_data['holding_duration'], cmap='viridis')
            axes[0, 0].set_xlabel('Holdings Value')
            axes[0, 0].set_ylabel('Reputation Score')
            axes[0, 0].set_title('Holdings Value vs Reputation\n(Color = Holding Duration)')
            axes[0, 0].set_xscale('log')
            plt.colorbar(scatter, ax=axes[0, 0], label='Holding Duration')
            
            # Add trend line
            if len(holdings_rep_data) > 10:
                correlation = holdings_rep_data['holdings_value'].corr(holdings_rep_data['reputation'])
                axes[0, 0].text(0.05, 0.95, f'Correlation: {correlation:.3f}', 
                              transform=axes[0, 0].transAxes, fontsize=10, 
                              bbox=dict(boxstyle="round,pad=0.3", facecolor="white", alpha=0.8))
        
        # 2. Buy Volume vs Reputation Scatter  
        buy_rep_data = self.data[(self.data['buy_volume'] > 0) & (self.data['reputation'] > 0)]
        if len(buy_rep_data) > 0:
            scatter = axes[0, 1].scatter(buy_rep_data['buy_volume'], buy_rep_data['reputation'], 
                                       alpha=0.6, c=buy_rep_data['volume'], cmap='plasma')
            axes[0, 1].set_xlabel('Buy Volume')
            axes[0, 1].set_ylabel('Reputation Score')
            axes[0, 1].set_title('Buy Volume vs Reputation\n(Color = Total Volume)')
            axes[0, 1].set_xscale('log')
            plt.colorbar(scatter, ax=axes[0, 1], label='Total Volume')
            
            if len(buy_rep_data) > 10:
                correlation = buy_rep_data['buy_volume'].corr(buy_rep_data['reputation'])
                axes[0, 1].text(0.05, 0.95, f'Correlation: {correlation:.3f}', 
                              transform=axes[0, 1].transAxes, fontsize=10,
                              bbox=dict(boxstyle="round,pad=0.3", facecolor="white", alpha=0.8))
        
        # 3. Reputation Distribution by Holdings Value Quartiles
        if len(holdings_rep_data) > 0:
            holdings_rep_data['holdings_quartile'] = pd.qcut(holdings_rep_data['holdings_value'], 
                                                           q=4, labels=['Q1', 'Q2', 'Q3', 'Q4'])
            quartile_rep = holdings_rep_data.groupby('holdings_quartile')['reputation'].agg(['mean', 'std'])
            
            axes[0, 2].bar(quartile_rep.index, quartile_rep['mean'], 
                          yerr=quartile_rep['std'], capsize=5, alpha=0.7)
            axes[0, 2].set_xlabel('Holdings Value Quartile')
            axes[0, 2].set_ylabel('Average Reputation')
            axes[0, 2].set_title('Average Reputation by\nHoldings Value Quartile')
        
        # 4. Reputation Distribution by Buy Volume Quartiles
        if len(buy_rep_data) > 0:
            buy_rep_data['buy_volume_quartile'] = pd.qcut(buy_rep_data['buy_volume'], 
                                                        q=4, labels=['Q1', 'Q2', 'Q3', 'Q4'])
            quartile_rep = buy_rep_data.groupby('buy_volume_quartile')['reputation'].agg(['mean', 'std'])
            
            axes[1, 0].bar(quartile_rep.index, quartile_rep['mean'], 
                          yerr=quartile_rep['std'], capsize=5, alpha=0.7)
            axes[1, 0].set_xlabel('Buy Volume Quartile')
            axes[1, 0].set_ylabel('Average Reputation')
            axes[1, 0].set_title('Average Reputation by\nBuy Volume Quartile')
        
        # 5. Combined Holdings + Buy Volume Effect (3D-like visualization)
        combined_data = self.data[(self.data['holdings_value'] > 0) & 
                                (self.data['buy_volume'] > 0) & 
                                (self.data['reputation'] > 0)]
        if len(combined_data) > 0:
            # Create bins for both metrics
            combined_data['holdings_bin'] = pd.qcut(combined_data['holdings_value'], q=5, labels=False)
            combined_data['buy_volume_bin'] = pd.qcut(combined_data['buy_volume'], q=5, labels=False)
            
            # Calculate average reputation for each combination
            heatmap_data = combined_data.groupby(['holdings_bin', 'buy_volume_bin'])['reputation'].mean().unstack()
            
            im = axes[1, 1].imshow(heatmap_data.values, cmap='RdYlGn', aspect='auto')
            axes[1, 1].set_xlabel('Buy Volume Quintile')
            axes[1, 1].set_ylabel('Holdings Value Quintile')
            axes[1, 1].set_title('Reputation Heatmap:\nHoldings vs Buy Volume')
            axes[1, 1].set_xticks(range(5))
            axes[1, 1].set_yticks(range(5))
            axes[1, 1].set_xticklabels(['Q1', 'Q2', 'Q3', 'Q4', 'Q5'])
            axes[1, 1].set_yticklabels(['Q1', 'Q2', 'Q3', 'Q4', 'Q5'])
            plt.colorbar(im, ax=axes[1, 1], label='Average Reputation')
        
        # 6. Top Performers Analysis
        if 'reputation' in self.data.columns:
            top_reputation = self.data.nlargest(100, 'reputation')
            
            # Compare top reputation users vs others
            others = self.data[~self.data.index.isin(top_reputation.index)]
            
            metrics_comparison = {
                'Holdings Value': [top_reputation['holdings_value'].mean(), others['holdings_value'].mean()],
                'Buy Volume': [top_reputation['buy_volume'].mean(), others['buy_volume'].mean()],
                'Total Volume': [top_reputation['volume'].mean(), others['volume'].mean()],
                'Duration': [top_reputation['holding_duration'].mean(), others['holding_duration'].mean()]
            }
            
            x = np.arange(len(metrics_comparison))
            width = 0.35
            
            top_values = [v[0] for v in metrics_comparison.values()]
            other_values = [v[1] for v in metrics_comparison.values()]
            
            axes[1, 2].bar(x - width/2, top_values, width, label='Top 100 Reputation', alpha=0.8)
            axes[1, 2].bar(x + width/2, other_values, width, label='Others', alpha=0.8)
            axes[1, 2].set_xlabel('Metrics')
            axes[1, 2].set_ylabel('Average Value (Log Scale)')
            axes[1, 2].set_title('Top Reputation Users vs Others')
            axes[1, 2].set_xticks(x)
            axes[1, 2].set_xticklabels(list(metrics_comparison.keys()), rotation=45)
            axes[1, 2].legend()
            axes[1, 2].set_yscale('log')
        
        plt.tight_layout()
        plt.show()
        
        # Print insights
        print("\n" + "="*70)
        print("REPUTATION IMPACT ANALYSIS INSIGHTS")
        print("="*70)
        
        if len(holdings_rep_data) > 0:
            holdings_correlation = holdings_rep_data['holdings_value'].corr(holdings_rep_data['reputation'])
            print(f"Holdings Value vs Reputation correlation: {holdings_correlation:.3f}")
            
        if len(buy_rep_data) > 0:
            buy_correlation = buy_rep_data['buy_volume'].corr(buy_rep_data['reputation'])
            print(f"Buy Volume vs Reputation correlation: {buy_correlation:.3f}")
            
        if 'reputation' in self.data.columns:
            top_rep_users = self.data.nlargest(10, 'reputation')
            print(f"\nTop 10 Users by Reputation:")
            for i, (_, user) in enumerate(top_rep_users.iterrows(), 1):
                print(f"{i:2d}. Reputation: {user['reputation']:8.0f} | "
                      f"Holdings: {user['holdings_value']:.2e} | "
                      f"Buy Vol: {user['buy_volume']:.2e}")
    
    def plot_volume_analysis(self, figsize=(15, 10)):
        """
        Detailed volume analysis charts
        """
        fig, axes = plt.subplots(2, 2, figsize=figsize)
        fig.suptitle('Trading Volume Analysis', fontsize=16)
        
        # 1. Buy vs Sell Volume Scatter
        buy_sell_data = self.data[(self.data['buy_volume'] > 0) | (self.data['sell_volume'] > 0)]
        if len(buy_sell_data) > 0:
            axes[0, 0].scatter(buy_sell_data['buy_volume'], buy_sell_data['sell_volume'], alpha=0.6)
            axes[0, 0].plot([0, buy_sell_data['buy_volume'].max()], [0, buy_sell_data['buy_volume'].max()], 
                           'r--', label='Equal Buy/Sell Line')
            axes[0, 0].set_xlabel('Buy Volume')
            axes[0, 0].set_ylabel('Sell Volume')
            axes[0, 0].set_title('Buy vs Sell Volume')
            axes[0, 0].legend()
            axes[0, 0].set_xscale('log')
            axes[0, 0].set_yscale('log')
        
        # 2. Total Volume Distribution (log scale)
        volume_data = self.data[self.data['volume'] > 0]['volume']
        if len(volume_data) > 0:
            axes[0, 1].hist(np.log10(volume_data), bins=50, alpha=0.7, edgecolor='black')
            axes[0, 1].set_xlabel('Log10(Total Volume)')
            axes[0, 1].set_ylabel('Frequency')
            axes[0, 1].set_title('Total Volume Distribution (Log Scale)')
        
        # 3. Volume vs Holdings Value
        vol_hold_data = self.data[(self.data['volume'] > 0) & (self.data['holdings_value'] > 0)]
        if len(vol_hold_data) > 0:
            axes[1, 0].scatter(vol_hold_data['volume'], vol_hold_data['holdings_value'], alpha=0.6)
            axes[1, 0].set_xlabel('Volume')
            axes[1, 0].set_ylabel('Holdings Value')
            axes[1, 0].set_title('Volume vs Holdings Value')
            axes[1, 0].set_xscale('log')
            axes[1, 0].set_yscale('log')
        
        # 4. Volume vs Reputation
        vol_rep_data = self.data[(self.data['volume'] > 0) & (self.data['reputation'] > 0)]
        if len(vol_rep_data) > 0:
            axes[1, 1].scatter(vol_rep_data['volume'], vol_rep_data['reputation'], alpha=0.6)
            axes[1, 1].set_xlabel('Volume')
            axes[1, 1].set_ylabel('Reputation Score')
            axes[1, 1].set_title('Volume vs Reputation')
            axes[1, 1].set_xscale('log')
            
            # Add correlation info
            correlation = vol_rep_data['volume'].corr(vol_rep_data['reputation'])
            axes[1, 1].text(0.05, 0.95, f'Correlation: {correlation:.3f}', 
                          transform=axes[1, 1].transAxes, fontsize=10,
                          bbox=dict(boxstyle="round,pad=0.3", facecolor="white", alpha=0.8))
        
        plt.tight_layout()
        plt.show()
    
    def plot_holding_analysis(self, figsize=(15, 8)):
        """
        Analyze holding duration and value patterns
        """
        fig, axes = plt.subplots(1, 3, figsize=figsize)
        fig.suptitle('Holding Pattern Analysis', fontsize=16)
        
        # 1. Holding Duration Distribution
        duration_data = self.data[self.data['holding_duration'] > 0]['holding_duration']
        if len(duration_data) > 0:
            axes[0].hist(duration_data, bins=50, alpha=0.7, edgecolor='black')
            axes[0].set_xlabel('Holding Duration')
            axes[0].set_ylabel('Frequency')
            axes[0].set_title('Holding Duration Distribution')
            axes[0].axvline(duration_data.mean(), color='red', linestyle='--', label=f'Mean: {duration_data.mean():.2f}')
            axes[0].legend()
        
        # 2. Holdings Value vs Duration
        hold_data = self.data[(self.data['holding_duration'] > 0) & (self.data['holdings_value'] > 0)]
        if len(hold_data) > 0:
            axes[1].scatter(hold_data['holding_duration'], hold_data['holdings_value'], alpha=0.6)
            axes[1].set_xlabel('Holding Duration')
            axes[1].set_ylabel('Holdings Value')
            axes[1].set_title('Holdings Value vs Duration')
            axes[1].set_yscale('log')
        
        # 3. Reputation vs Duration
        rep_duration_data = self.data[(self.data['holding_duration'] > 0) & (self.data['reputation'] > 0)]
        if len(rep_duration_data) > 0:
            # Create duration bins
            duration_bins = pd.cut(rep_duration_data['holding_duration'], bins=10)
            avg_rep_by_duration = rep_duration_data.groupby(duration_bins)['reputation'].mean()
            
            axes[2].bar(range(len(avg_rep_by_duration)), avg_rep_by_duration.values, alpha=0.7)
            axes[2].set_xlabel('Holding Duration Bins')
            axes[2].set_ylabel('Average Reputation Score')
            axes[2].set_title('Reputation Score by Duration')
            axes[2].tick_params(axis='x', rotation=45)
            
            # Add correlation info
            correlation = rep_duration_data['holding_duration'].corr(rep_duration_data['reputation'])
            axes[2].text(0.05, 0.95, f'Correlation: {correlation:.3f}', 
                        transform=axes[2].transAxes, fontsize=10,
                        bbox=dict(boxstyle="round,pad=0.3", facecolor="white", alpha=0.8))
        
        plt.tight_layout()
        plt.show()
    
    def plot_reputation_ranking_validation(self, figsize=(15, 12)):
        """
        Validate the reputation ranking algorithm
        """
        # Calculate composite reputation score (you can adjust weights)
        # Note: Using reputation as primary metric, with supporting factors
        weights = {
            'reputation': 0.4,
            'volume': 0.25,
            'holding_duration': 0.2,
            'holdings_value': 0.15
        }
        
        # Normalize metrics (0-1 scale)
        normalized_data = self.data.copy()
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
        
        fig, axes = plt.subplots(2, 2, figsize=figsize)
        fig.suptitle('Reputation Ranking Algorithm Validation', fontsize=16)
        
        # 1. Composite Score Distribution
        axes[0, 0].hist(normalized_data['composite_score'], bins=50, alpha=0.7, edgecolor='black')
        axes[0, 0].set_xlabel('Composite Reputation Score')
        axes[0, 0].set_ylabel('Frequency')
        axes[0, 0].set_title('Composite Score Distribution')
        
        # 2. Top performers analysis
        top_100 = normalized_data.nsmallest(100, 'rank')
        bottom_100 = normalized_data.nlargest(100, 'rank')
        
        metrics_to_compare = ['reputation', 'volume', 'holding_duration', 'holdings_value']
        top_means = [top_100[m].mean() for m in metrics_to_compare if m in top_100.columns]
        bottom_means = [bottom_100[m].mean() for m in metrics_to_compare if m in bottom_100.columns]
        
        x = np.arange(len(metrics_to_compare))
        width = 0.35
        
        axes[0, 1].bar(x - width/2, top_means, width, label='Top 100', alpha=0.7)
        axes[0, 1].bar(x + width/2, bottom_means, width, label='Bottom 100', alpha=0.7)
        axes[0, 1].set_xlabel('Metrics')
        axes[0, 1].set_ylabel('Average Value')
        axes[0, 1].set_title('Top vs Bottom Performers')
        axes[0, 1].set_xticks(x)
        axes[0, 1].set_xticklabels(metrics_to_compare, rotation=45)
        axes[0, 1].legend()
        axes[0, 1].set_yscale('log')
        
        # 3. Rank vs Individual Metrics
        sample_data = normalized_data.sample(min(1000, len(normalized_data)))
        
        axes[1, 0].scatter(sample_data['rank'], sample_data['volume'], alpha=0.6)
        axes[1, 0].set_xlabel('Rank')
        axes[1, 0].set_ylabel('Volume')
        axes[1, 0].set_title('Rank vs Volume')
        axes[1, 0].set_yscale('log')
        
        # 4. Score components contribution
        if len(top_100) > 0:
            score_components = []
            labels = []
            for metric, weight in weights.items():
                if f'{metric}_norm' in top_100.columns:
                    contribution = (top_100[f'{metric}_norm'] * weight).mean()
                    score_components.append(contribution)
                    labels.append(metric.replace('_', ' ').title())
            
            axes[1, 1].pie(score_components, labels=labels, autopct='%1.1f%%')
            axes[1, 1].set_title('Score Components (Top 100 Users)')
        
        plt.tight_layout()
        plt.show()
        
        # Print ranking insights
        print("\n" + "="*60)
        print("RANKING ALGORITHM INSIGHTS")
        print("="*60)
        print(f"Total users analyzed: {len(normalized_data)}")
        print(f"Users with composite score > 0: {len(normalized_data[normalized_data['composite_score'] > 0])}")
        print(f"\nTop 10 Users by Composite Score:")
        top_10 = normalized_data.nsmallest(10, 'rank')[['id', 'composite_score', 'rank'] + list(weights.keys())]
        print(top_10.to_string(index=False))
        
        return normalized_data
    
    def plot_outlier_detection(self, figsize=(15, 10)):
        """
        Detect and visualize outliers in the data
        """
        fig, axes = plt.subplots(2, 2, figsize=figsize)
        fig.suptitle('Outlier Detection Analysis', fontsize=16)
        
        metrics = ['volume', 'holding_duration', 'holdings_value', 'diamond_hand_probability']
        
        for i, metric in enumerate(metrics):
            if metric in self.data.columns:
                row, col = i // 2, i % 2
                ax = axes[row, col]
                
                data_clean = self.data[self.data[metric] > 0][metric]
                
                if len(data_clean) > 0:
                    # Box plot
                    box_plot = ax.boxplot(data_clean, patch_artist=True)
                    box_plot['boxes'][0].set_facecolor('lightblue')
                    
                    # Calculate outliers using IQR method
                    Q1 = data_clean.quantile(0.25)
                    Q3 = data_clean.quantile(0.75)
                    IQR = Q3 - Q1
                    lower_bound = Q1 - 1.5 * IQR
                    upper_bound = Q3 + 1.5 * IQR
                    
                    outliers = data_clean[(data_clean < lower_bound) | (data_clean > upper_bound)]
                    
                    ax.set_title(f'{metric.replace("_", " ").title()}\n{len(outliers)} outliers ({len(outliers)/len(data_clean)*100:.1f}%)')
                    ax.set_ylabel('Value')
                    
                    if data_clean.max() / data_clean.min() > 1000:  # Use log scale for high variance
                        ax.set_yscale('log')
        
        plt.tight_layout()
        plt.show()
    
    def plot_temporal_analysis(self, figsize=(15, 8)):
        """
        Analyze temporal patterns in trading behavior
        """
        if 'first_bought' not in self.data.columns:
            print("No temporal data available for analysis")
            return
        
        temporal_data = self.data.dropna(subset=['first_bought'])
        temporal_data['date'] = temporal_data['first_bought'].dt.date
        temporal_data['hour'] = temporal_data['first_bought'].dt.hour
        temporal_data['day_of_week'] = temporal_data['first_bought'].dt.day_name()
        
        fig, axes = plt.subplots(1, 3, figsize=figsize)
        fig.suptitle('Temporal Trading Patterns', fontsize=16)
        
        # 1. Trading activity by date
        daily_activity = temporal_data.groupby('date').size()
        axes[0].plot(daily_activity.index, daily_activity.values)
        axes[0].set_xlabel('Date')
        axes[0].set_ylabel('Number of First Purchases')
        axes[0].set_title('Daily Trading Activity')
        axes[0].tick_params(axis='x', rotation=45)
        
        # 2. Trading activity by hour
        hourly_activity = temporal_data.groupby('hour').size()
        axes[1].bar(hourly_activity.index, hourly_activity.values, alpha=0.7)
        axes[1].set_xlabel('Hour of Day')
        axes[1].set_ylabel('Number of First Purchases')
        axes[1].set_title('Hourly Trading Activity')
        
        # 3. Trading activity by day of week
        daily_activity_dow = temporal_data.groupby('day_of_week').size()
        day_order = ['Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday', 'Sunday']
        daily_activity_dow = daily_activity_dow.reindex(day_order, fill_value=0)
        axes[2].bar(daily_activity_dow.index, daily_activity_dow.values, alpha=0.7)
        axes[2].set_xlabel('Day of Week')
        axes[2].set_ylabel('Number of First Purchases')
        axes[2].set_title('Weekly Trading Activity')
        axes[2].tick_params(axis='x', rotation=45)
        
        plt.tight_layout()
        plt.show()
    
    def generate_algorithm_report(self):
        """
        Generate a comprehensive report on algorithm performance
        """
        print("\n" + "="*80)
        print("TRADING REPUTATION ALGORITHM ANALYSIS REPORT")
        print("="*80)
        
        # Data quality metrics
        print("\n1. DATA QUALITY METRICS:")
        print("-" * 40)
        total_users = len(self.data)
        active_traders = len(self.data[self.data['volume'] > 0])
        holders = len(self.data[self.data['holdings_value'] > 0])
        
        print(f"Total users: {total_users:,}")
        print(f"Active traders (volume > 0): {active_traders:,} ({active_traders/total_users*100:.1f}%)")
        print(f"Current holders (holdings > 0): {holders:,} ({holders/total_users*100:.1f}%)")
        
        # Distribution insights
        print("\n2. METRIC DISTRIBUTIONS:")
        print("-" * 40)
        for metric in ['reputation', 'volume', 'holding_duration', 'holdings_value']:
            if metric in self.data.columns:
                data_clean = self.data[self.data[metric] > 0][metric]
                if len(data_clean) > 0:
                    print(f"{metric.replace('_', ' ').title()}:")
                    print(f"  - Users with data: {len(data_clean):,}")
                    print(f"  - Mean: {data_clean.mean():.2e}")
                    print(f"  - Median: {data_clean.median():.2e}")
                    print(f"  - Std Dev: {data_clean.std():.2e}")
                    print(f"  - Min: {data_clean.min():.2e}")
                    print(f"  - Max: {data_clean.max():.2e}")
        
        # Algorithm effectiveness
        print("\n3. ALGORITHM EFFECTIVENESS:")
        print("-" * 40)
        
        # Check if high-volume traders have high reputation
        if 'volume' in self.data.columns and 'reputation' in self.data.columns:
            high_volume = self.data[self.data['volume'] > self.data['volume'].quantile(0.9)]
            avg_reputation_high_vol = high_volume['reputation'].mean()
            avg_reputation_overall = self.data['reputation'].mean()
            
            print(f"Average reputation (top 10% volume traders): {avg_reputation_high_vol:.2f}")
            print(f"Average reputation (all users): {avg_reputation_overall:.2f}")
            print(f"High-volume trader premium: {(avg_reputation_high_vol/avg_reputation_overall-1)*100:.1f}%")
            
        # Check correlation with holdings
        if 'holdings_value' in self.data.columns and 'reputation' in self.data.columns:
            high_holdings = self.data[self.data['holdings_value'] > self.data['holdings_value'].quantile(0.9)]
            avg_reputation_high_holdings = high_holdings['reputation'].mean()
            
            print(f"Average reputation (top 10% holders): {avg_reputation_high_holdings:.2f}")
            print(f"High-holders premium: {(avg_reputation_high_holdings/avg_reputation_overall-1)*100:.1f}%")
            
        # Check correlation with buy volume
        if 'buy_volume' in self.data.columns and 'reputation' in self.data.columns:
            high_buy_volume = self.data[self.data['buy_volume'] > self.data['buy_volume'].quantile(0.9)]
            avg_reputation_high_buy = high_buy_volume['reputation'].mean()
            
            print(f"Average reputation (top 10% buy volume): {avg_reputation_high_buy:.2f}")
            print(f"High-buy-volume premium: {(avg_reputation_high_buy/avg_reputation_overall-1)*100:.1f}%")
        
        print("\n4. RECOMMENDATIONS:")
        print("-" * 40)
        print("- Monitor for data quality issues (many users with zero values)")
        print("- Consider adjusting algorithm weights based on correlation analysis")
        print("- Implement outlier detection to prevent manipulation")
        print("- Validate temporal patterns for suspicious activity")
        print("- Consider additional metrics like trade frequency or profit/loss")

def main():
    """
    Main function to run the complete analysis
    """
    # Initialize analyzer
    analyzer = TradingReputationAnalyzer('cult_data_aug_8.csv')
    
    # Run comprehensive analysis
    print("Starting Trading Reputation Algorithm Analysis...")
    
    # Data overview
    analyzer.data_overview()
    
    # Generate all charts
    print("\nGenerating distribution charts...")
    analyzer.plot_reputation_distributions()
    
    print("\nGenerating reputation impact analysis...")
    analyzer.plot_reputation_impact_analysis()
    
    print("\nGenerating correlation analysis...")
    analyzer.plot_correlation_matrix()
    
    print("\nGenerating volume analysis...")
    analyzer.plot_volume_analysis()
    
    print("\nGenerating holding pattern analysis...")
    analyzer.plot_holding_analysis()
    
    print("\nGenerating ranking validation...")
    ranking_data = analyzer.plot_reputation_ranking_validation()
    
    print("\nGenerating outlier detection...")
    analyzer.plot_outlier_detection()
    
    print("\nGenerating temporal analysis...")
    analyzer.plot_temporal_analysis()
    
    # Generate final report
    analyzer.generate_algorithm_report()
    
    print("\nAnalysis complete! Review the charts and report for algorithm insights.")
    
    return analyzer

if __name__ == "__main__":
    analyzer = main()
