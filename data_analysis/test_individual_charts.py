#!/usr/bin/env python3
"""
Individual Chart Testing Script
This script allows you to test individual analysis functions.
"""

from analyze import TradingReputationAnalyzer
import matplotlib.pyplot as plt

def test_individual_charts():
    """
    Test individual chart functions for specific analysis needs
    """
    print("Testing individual chart functions...")
    
    # Initialize analyzer
    analyzer = TradingReputationAnalyzer('cult_data_aug_8.csv')
    
    print("\nAvailable chart functions:")
    print("1. Distribution charts - plot_reputation_distributions()")
    print("2. Reputation impact analysis - plot_reputation_impact_analysis()")
    print("3. Correlation matrix - plot_correlation_matrix()")
    print("4. Volume analysis - plot_volume_analysis()")
    print("5. Holding analysis - plot_holding_analysis()")
    print("6. Ranking validation - plot_reputation_ranking_validation()")
    print("7. Outlier detection - plot_outlier_detection()")
    print("8. Temporal analysis - plot_temporal_analysis()")
    print("9. Generate report - generate_algorithm_report()")
    
    return analyzer

def run_specific_analysis(analyzer, analysis_type):
    """
    Run a specific type of analysis
    
    Args:
        analyzer: TradingReputationAnalyzer instance
        analysis_type: String indicating which analysis to run
    """
    
    if analysis_type == "distributions":
        print("\n📊 Running distribution analysis...")
        analyzer.plot_reputation_distributions()
        
    elif analysis_type == "reputation_impact":
        print("\n🎯 Running reputation impact analysis...")
        analyzer.plot_reputation_impact_analysis()
        
    elif analysis_type == "correlations":
        print("\n🔗 Running correlation analysis...")
        analyzer.plot_correlation_matrix()
        
    elif analysis_type == "volume":
        print("\n💰 Running volume analysis...")
        analyzer.plot_volume_analysis()
        
    elif analysis_type == "holdings":
        print("\n💎 Running holding pattern analysis...")
        analyzer.plot_holding_analysis()
        
    elif analysis_type == "ranking":
        print("\n🏆 Running ranking validation...")
        ranking_data = analyzer.plot_reputation_ranking_validation()
        return ranking_data
        
    elif analysis_type == "outliers":
        print("\n⚠️ Running outlier detection...")
        analyzer.plot_outlier_detection()
        
    elif analysis_type == "temporal":
        print("\n⏰ Running temporal analysis...")
        analyzer.plot_temporal_analysis()
        
    elif analysis_type == "report":
        print("\n📋 Generating algorithm report...")
        analyzer.generate_algorithm_report()
        
    else:
        print(f"❌ Unknown analysis type: {analysis_type}")
        print("Available types: distributions, reputation_impact, correlations, volume, holdings, ranking, outliers, temporal, report")

# Example usage functions
def example_quick_distribution_check():
    """Example: Quick check of data distributions"""
    analyzer = TradingReputationAnalyzer('cult_data_aug_8.csv')
    
    # Just run distribution analysis
    analyzer.plot_reputation_distributions()
    
    print("✅ Distribution analysis complete!")
    print("Check the charts to see if your data looks reasonable:")
    print("- Are there clear patterns in volume/holdings distributions?")
    print("- Do reputation scores show expected ranges?")
    print("- Are there significant outliers to investigate?")

def example_reputation_impact_analysis():
    """Example: Focus on how holdings and buy volume affect reputation"""
    analyzer = TradingReputationAnalyzer('cult_data_aug_8.csv')
    
    # Run reputation impact analysis
    analyzer.plot_reputation_impact_analysis()
    
    print("✅ Reputation impact analysis complete!")
    print("Key things to check:")
    print("1. Do holdings value and reputation show positive correlation?")
    print("2. Does buy volume correlate with higher reputation?")
    print("3. Are high-reputation users clustered in specific holdings/volume ranges?")
    print("4. Does the heatmap show clear patterns of reputation distribution?")

def example_algorithm_validation():
    """Example: Focus on validating the ranking algorithm"""
    analyzer = TradingReputationAnalyzer('cult_data_aug_8.csv')
    
    # Run correlation analysis first
    analyzer.plot_correlation_matrix()
    
    # Then ranking validation
    ranking_data = analyzer.plot_reputation_ranking_validation()
    
    # Generate report
    analyzer.generate_algorithm_report()
    
    print("✅ Algorithm validation complete!")
    print("Key things to check:")
    print("1. Are high-volume traders getting higher scores?")
    print("2. Do correlations make sense for your business logic?")
    print("3. Are the top-ranked users actually the best traders?")
    
    return ranking_data

def example_data_quality_check():
    """Example: Focus on data quality issues"""
    analyzer = TradingReputationAnalyzer('cult_data_aug_8.csv')
    
    # Check outliers
    analyzer.plot_outlier_detection()
    
    # Look at temporal patterns for anomalies
    analyzer.plot_temporal_analysis()
    
    # Generate report for summary
    analyzer.generate_algorithm_report()
    
    print("✅ Data quality check complete!")
    print("Look for:")
    print("1. Unusual outliers that might indicate manipulation")
    print("2. Temporal patterns that seem suspicious")
    print("3. Data quality issues in the report")

if __name__ == "__main__":
    # Example usage
    print("="*60)
    print("INDIVIDUAL CHART TESTING")
    print("="*60)
    
    # Initialize
    analyzer = test_individual_charts()
    
    print(f"\n🔍 Example usage:")
    print(f"# Quick distribution check")
    print(f"example_quick_distribution_check()")
    print(f"")
    print(f"# Reputation impact analysis")
    print(f"example_reputation_impact_analysis()")
    print(f"")
    print(f"# Algorithm validation")
    print(f"example_algorithm_validation()")
    print(f"")
    print(f"# Data quality check")
    print(f"example_data_quality_check()")
    print(f"")
    print(f"# Run specific analysis")
    print(f"run_specific_analysis(analyzer, 'reputation_impact')")
    
    print(f"\n💡 Uncomment one of the examples below to test:")
    print(f"")
    # example_quick_distribution_check()
    # example_algorithm_validation()
    # example_data_quality_check()
    
    print("Script ready for testing individual analyses!") 